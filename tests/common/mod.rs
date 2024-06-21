use anyhow::Error;

use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use scouter_server::kafka::consumer::{setup_kafka_consumer, MessageHandler};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
use std::env;
use std::time::Duration;

use rdkafka::error::KafkaError;

pub async fn setup_db() -> Result<PostgresClient, Error> {
    // set the postgres database url
    env::set_var(
        "DATABASE_URL",
        "postgresql://postgres:admin@localhost:5432/monitor?",
    );

    // set the max connections for the postgres pool
    env::set_var("MAX_CONNECTIONS", "10");

    let client = PostgresClient::new(None).await.expect("error");

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

    Ok(client)
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
    let db_client = setup_db().await.unwrap();
    let message_handler = MessageHandler::Postgres(db_client.clone());

    let producer_task = tokio::spawn(async move {
        let producer: &FutureProducer = &ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .create()
            .expect("Producer creation error");
        let feature_names = vec!["feature1", "feature2", "feature3"];

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
        setup_kafka_consumer(
            "scouter".to_string(),
            "localhost:9092".to_string(),
            vec!["scouter_monitoring".to_string()],
            message_handler,
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

    let db_client = setup_db().await.unwrap();

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&db_client.pool)
    .await
    .unwrap();

    Ok(())
}
