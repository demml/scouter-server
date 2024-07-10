use crate::test_utils::*;
use anyhow::Context;

use scouter_server::kafka::consumer::{start_kafka_background_poll, MessageHandler};
use scouter_server::sql::postgres::PostgresClient;

mod test_utils;

#[tokio::test(flavor = "multi_thread")]
#[ignore]
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
}
