use crate::sql::postgres::PostgresClient;
use crate::sql::schema::DriftRecord;
use anyhow::*;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;

use futures::StreamExt;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::CommitMode;
use rdkafka::message::BorrowedMessage;

use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use rdkafka::Message;
use std::result::Result::Ok;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;
use tracing::info;

// Get table name constant

pub enum MessageHandler {
    Postgres(PostgresClient),
}

impl MessageHandler {
    pub async fn insert_drift_record(&self, records: &DriftRecord) -> Result<()> {
        match self {
            Self::Postgres(client) => {
                let result = client.insert_drift_record(records).await;
                match result {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Failed to insert drift record: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

pub async fn create_kafka_consumer(
    group_id: String,
    brokers: String,
    topics: Vec<String>,
    username: Option<String>,
    password: Option<String>,
    security_protocol: Option<String>,
    sasl_mechanism: Option<String>,
) -> Result<StreamConsumer, anyhow::Error> {
    info!("Setting up Kafka consumer");

    let mut config = ClientConfig::new();

    config
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true");

    if username.is_some() && password.is_some() {
        config
            .set("security.protocol", security_protocol.unwrap())
            .set("sasl.mechanisms", sasl_mechanism.unwrap())
            .set("sasl.username", username.unwrap())
            .set("sasl.password", password.unwrap());
    }

    let consumer: StreamConsumer = config.create().expect("Consumer creation error");

    let topics = topics.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

    consumer
        .subscribe(&topics)
        .expect("Can't subscribe to specified topics");

    Ok(consumer)
}

#[allow(clippy::unnecessary_unwrap)]
#[allow(clippy::too_many_arguments)]
pub async fn stream_from_kafka_topic(
    message_handler: &MessageHandler,
    consumer: &StreamConsumer,
) -> Result<(), anyhow::Error> {
    let mut stream = consumer.stream();
    let message = stream.next().await;

    match message {
        Some(Ok(msg)) => {
            let payload = msg.payload().unwrap();
            let record: DriftRecord = serde_json::from_slice(payload).unwrap();
            println!("Received record: {:?}", record);
            let inserted = message_handler.insert_drift_record(&record).await;
            match inserted {
                Ok(_) => {
                    consumer.commit_message(&msg, CommitMode::Async).unwrap();
                }
                Err(e) => {
                    error!("Failed to insert drift record: {:?}", e);
                }
            }
        }
        Some(Err(e)) => {
            error!("Failed to receive message: {:?}", e);
        }
        None => {
            error!("No message received");
        }
    }

    Ok(())
}

#[allow(clippy::unnecessary_unwrap)]
#[allow(clippy::too_many_arguments)]
pub async fn start_kafka_background_poll(
    message_handler: MessageHandler,
    group_id: String,
    brokers: String,
    topics: Vec<String>,
    username: Option<String>,
    password: Option<String>,
    security_protocol: Option<String>,
    sasl_mechanism: Option<String>,
) -> Result<(), anyhow::Error> {
    let consumer = create_kafka_consumer(
        group_id,
        brokers,
        topics,
        username,
        password,
        security_protocol,
        sasl_mechanism,
    )
    .await
    .unwrap();

    loop {
        stream_from_kafka_topic(&message_handler, &consumer).await?;
    }
}
