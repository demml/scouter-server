use anyhow::Error;
use axum::Router;
use lapin::BasicProperties;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
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

#[allow(dead_code)]
pub async fn populate_topic(topic_name: &str) -> Result<(), Error> {
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

    for i in 0..15 {
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

            let produce_future = producer.send(
                FutureRecord::to(topic_name)
                    .payload(&record_string)
                    .key("Key"),
                Duration::from_secs(1),
            );

            match produce_future.await {
                Ok(delivery) => println!("Sent: {:?}", delivery),
                Err((e, _)) => println!("Error: {:?}", e),
            }
        }
    }
    producer.flush(Duration::from_secs(1)).unwrap();
    Ok(())
}

#[allow(dead_code)]
pub async fn populate_rabbit_queue() -> Result<(), Error> {
    // Produce some messages

    let rabbit_addr = std::env::var("RABBITMQ_ADDR")
        .unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into());

    let conn = Connection::connect(&rabbit_addr, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await.unwrap();

    channel
        .queue_declare(
            "scouter_monitoring",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    for i in 0..15 {
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

            let record_string = serde_json::to_string(&record).unwrap().into_bytes();

            let _confirm = channel
                .basic_publish(
                    "",
                    "scouter_monitoring",
                    BasicPublishOptions::default(),
                    &record_string,
                    BasicProperties::default(),
                )
                .await?;
        }
    }

    Ok(())
}

pub async fn setup_db(clean_db: bool) -> Result<Pool<Postgres>, Error> {
    // set the postgres database url
    unsafe {
        env::set_var(
            "DATABASE_URL",
            "postgresql://postgres:admin@localhost:5432/monitor?",
        );
        env::set_var("MAX_CONNECTIONS", "10");
    }

    // set the max connections for the postgres pool
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
            FROM scouter.drift;

            DELETE 
            FROM scouter.drift_profile;
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
            FROM scouter.drift;

            DELETE 
            FROM scouter.drift_profile;

            DELETE
            FROM scouter.drift_alerts;
            "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Ok(())
}
