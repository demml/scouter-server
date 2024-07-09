use scouter_server::kafka::consumer::{setup_kafka_consumer, MessageHandler};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;

use tokio;
mod common;

use rdkafka::config::ClientConfig;

use rdkafka::producer::FutureProducer;

use common::produce_message;
#[tokio::test]
async fn test_scouter_consumer() {
    // set consumer

    let (db_client, pool) = common::setup_test_db().await.unwrap();

    let message_handler = MessageHandler::Postgres(db_client.clone());

    let consumer_task = tokio::spawn(async move {
        setup_kafka_consumer(
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

    let producer_task = tokio::spawn(async move {
        let producer: &FutureProducer = &ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .create()
            .expect("Producer creation error");

        for i in 0..10 {
            let record = DriftRecord {
                created_at: chrono::Utc::now().naive_utc(),
                name: "test_app".to_string(),
                repository: "test".to_string(),
                feature: "test".to_string(),
                value: i as f64,
                version: "1.0.0".to_string(),
            };

            let record_string = serde_json::to_string(&record).unwrap();
            produce_message(&record_string, producer).await.unwrap();
        }
    });

    // wait for 5 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    producer_task.abort();
    consumer_task.abort();

    let results = db_client
        .raw_query(
            r#"
        SELECT * 
        FROM scouter.drift  
        WHERE name = 'test_app'
        "#,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 10);

    // teardown
    common::teardown().await.unwrap();
}
