use anyhow::Error;

use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use scouter_server::api::setup::setup_db;
use scouter_server::kafka::consumer::{start_kafka_background_poll, MessageHandler};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
use sqlx::postgres::Postgres;
use sqlx::Pool;
use std::env;
use std::time::Duration;

use rdkafka::error::KafkaError;

pub async fn setup_test_db() -> Result<(PostgresClient, Pool<Postgres>), Error> {
    // set the postgres database url
    env::set_var(
        "DATABASE_URL",
        "postgresql://postgres:admin@localhost:5432/monitor?",
    );

    // set the max connections for the postgres pool
    env::set_var("MAX_CONNECTIONS", "10");
    let pool = setup_db(None).await.expect("error");

    // run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let client = PostgresClient::new(pool.clone()).unwrap();

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&client.pool)
    .await
    .unwrap();

    Ok((client, pool))
}

#[allow(dead_code)]
pub async fn produce_message(message: &str, producer: &FutureProducer) -> Result<(), KafkaError> {
    producer
        .send(
            FutureRecord::to("scouter_monitoring")
                .payload(message)
                .key("Key"),
            Duration::from_secs(0),
        )
        .await
        .unwrap();
    Ok(())
}

#[allow(dead_code)]
pub async fn setup_for_api() -> Result<
    (
        PostgresClient,
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
    ),
    Error,
> {
    let (db_client, _pool) = setup_test_db().await.unwrap();

    let message_handler = MessageHandler::Postgres(db_client.clone());

    let producer_task = tokio::spawn(async move {
        let producer: &FutureProducer = &ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .create()
            .expect("Producer creation error");
        let feature_names = vec!["feature0", "feature1", "feature2"];

        for feature_name in feature_names {
            for i in 0..100 {
                let record = DriftRecord {
                    created_at: chrono::Utc::now().naive_utc(),
                    name: "test_app".to_string(),
                    repository: "test".to_string(),
                    feature: feature_name.to_string(),
                    value: i as f64,
                    version: "1.0.0".to_string(),
                };

                let record_string = serde_json::to_string(&record).unwrap();
                produce_message(&record_string, producer).await.unwrap();
            }
        }
    });

    let consumer_task = tokio::spawn(async move {
        start_kafka_background_poll(
            message_handler,
            "scouter".to_string(),
            "localhost:9092".to_string(),
            vec!["scouter_monitoring".to_string()],
            None,
            None,
            None,
            None,
        )
        .await;
    });

    // wait for 5 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    Ok((db_client, producer_task, consumer_task))
}

#[allow(dead_code)]
pub async fn teardown() -> Result<(), Error> {
    // clear the database

    let (_, pool) = setup_test_db().await.unwrap();

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
