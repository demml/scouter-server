use anyhow::Context;
use futures::future;
use rdkafka::Message;
use scouter_server::kafka::consumer::{
    create_kafka_consumer, start_kafka_background_poll, MessageHandler,
};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
mod common;
use crate::utils::*;
use common::produce_message;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::{BaseConsumer, StreamConsumer};
use rdkafka::producer::FutureProducer;
mod utils;

#[tokio::test]
#[ignore]
async fn test_scouter_consumer() {
    // set consumer

    let (db_client, pool) = common::setup_test_db().await.unwrap();

    // set env vars
    std::env::set_var("KAFKA_BROKER", "localhost:9092");
    std::env::set_var("KAFKA_TOPIC", "scouter_monitoring");
    std::env::set_var("KAFKA_GROUP", "scouter");

    // setup background task if kafka is enabled
    // this reproduces main.rs logic for bakcground kafka consumer
    if std::env::var("KAFKA_BROKER").is_ok() {
        let brokers = std::env::var("KAFKA_BROKER").unwrap();
        let topics = vec![std::env::var("KAFKA_TOPIC").unwrap()];
        let group_id = std::env::var("KAFKA_GROUP").unwrap();
        let username: Option<String> = std::env::var("KAFKA_USERNAME").ok();
        let password: Option<String> = std::env::var("KAFKA_PASSWORD").ok();
        let security_protocol: Option<String> = Some(
            std::env::var("KAFKA_SECURITY_PROTOCOL")
                .ok()
                .unwrap_or_else(|| "SASL_SSL".to_string()),
        );
        let sasl_mechanism: Option<String> = Some(
            std::env::var("KAFKA_SASL_MECHANISM")
                .ok()
                .unwrap_or_else(|| "PLAIN".to_string()),
        );

        let _background = (0..1)
            .map(|_| {
                let db_client = PostgresClient::new(pool.clone())
                    .with_context(|| "Failed to create Postgres client")
                    .unwrap();
                let message_handler = MessageHandler::Postgres(db_client);
                tokio::spawn(start_kafka_background_poll(
                    message_handler,
                    group_id.clone(),
                    brokers.clone(),
                    topics.clone(),
                    username.clone(),
                    password.clone(),
                    security_protocol.clone(),
                    sasl_mechanism.clone(),
                ))
            })
            .collect::<FuturesUnordered<_>>()
            .for_each(|_| async {});
    }

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

    // wait for 5 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

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

#[tokio::test]
async fn test_produce_consume_base() {
    let topic_name = "scouter_monitoring";

    populate_topic(topic_name).await;

    let config = utils::consumer_config("scouter", None);
    let consumer: StreamConsumer = config.create().expect("Consumer creation error");
    consumer.subscribe(&[topic_name]).unwrap();

    consumer
        .stream()
        .take(3)
        .for_each(|message| async {
            let message = message.unwrap();
            let payload = message.payload_view::<str>().unwrap().unwrap();
            println!("Message payload: {}", payload);
        })
        .await;
}
