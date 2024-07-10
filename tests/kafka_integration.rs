use crate::test_utils::*;
use anyhow::Context;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use scouter_server::kafka::consumer::{
    create_kafka_consumer, start_kafka_background_poll, stream_from_kafka_topic, MessageHandler,
};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::QueryResult;
use serde_json::Value;
use std::collections::HashMap;
use tower::Service;

mod test_utils;

#[tokio::test(flavor = "multi_thread")]
async fn test_api_with_kafka() {
    // setup resources
    let topic_name = "scouter_monitoring";
    let pool = test_utils::setup_db(true).await.unwrap();

    let message_handler = MessageHandler::Postgres(
        PostgresClient::new(pool.clone())
            .with_context(|| "Failed to create Postgres client")
            .unwrap(),
    );

    // consume 15 messages
    tokio::spawn(
        async move {
            start_kafka_background_poll(
                message_handler,
                "scouter".to_string(),
                "localhost:9092".to_string(),
                [topic_name.to_string()].to_vec(),
                None,
                None,
                None,
                None,
            )
        }
        .await,
    );

    // populate kafka topic (15 messages)
    populate_topic(topic_name).await;

    // consumer from the topic and write to database
    let mut config_overrides = HashMap::new();
    config_overrides.insert("auto.offset.reset", "earliest");
}
