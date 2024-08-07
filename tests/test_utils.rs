use anyhow::Error;
use axum::Router;
use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::producer::Producer;
use scouter_server::api::route::create_router;
use scouter_server::api::route::AppState;
use scouter_server::api::setup::create_db_pool;
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
use sqlx::Pool;
use sqlx::Postgres;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[allow(dead_code)]
pub async fn populate_topic(topic_name: &str) {
    // Produce some messages

    let kafka_brokers = env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_owned());
    let producer: &FutureProducer = &ClientConfig::new()
        .set("bootstrap.servers", &kafka_brokers)
        .set("statistics.interval.ms", "500")
        .set("api.version.request", "true")
        .set("debug", "all")
        .set("message.timeout.ms", "30000")
        .create()
        .expect("Producer creation error");

    let _ = (0..5)
        .map(|i| async move {
            // The send operation on the topic returns a future, which will be
            // completed once the result or failure from Kafka is received.
            let feature_names = vec!["feature0", "feature1", "feature2"];

            for feature_name in feature_names {
                let record = DriftRecord {
                    created_at: chrono::Utc::now().naive_utc(),
                    name: "test_app".to_string(),
                    repository: "test".to_string(),
                    feature: feature_name.to_string(),
                    value: i as f64,
                    version: "1.0.0".to_string(),
                };

                let record_string = serde_json::to_string(&record).unwrap();

                let _ = producer
                    .send(
                        FutureRecord::to(topic_name)
                            .payload(&record_string)
                            .key("Key"),
                        Duration::from_secs(0),
                    )
                    .await;
            }

            // This will be executed when the result is received.
            info!("Delivery status for message {} received", i);
            i
        })
        .collect::<Vec<_>>();

    producer.flush(Duration::from_secs(1)).unwrap()
}

pub async fn setup_db(clean_db: bool) -> Result<Pool<Postgres>, Error> {
    // set the postgres database url
    env::set_var(
        "DATABASE_URL",
        "postgresql://postgres:admin@localhost:5432/monitor?",
    );

    // set the max connections for the postgres pool
    env::set_var("MAX_CONNECTIONS", "10");
    let pool = create_db_pool(None).await.expect("error");

    // run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    if clean_db {
        sqlx::raw_sql(
            r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
    }

    Ok(pool)
}

#[allow(dead_code)]
pub async fn setup_api(clean_db: bool) -> Result<Router, Error> {
    // set the postgres database url
    let pool = setup_db(clean_db).await.unwrap();

    let db_client = PostgresClient::new(pool).unwrap();
    let router = create_router(Arc::new(AppState { db: db_client }));

    Ok(router)
}

#[allow(dead_code)]
pub async fn teardown() -> Result<(), Error> {
    // clear the database

    let pool = setup_db(true).await.unwrap();

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Ok(())
}
