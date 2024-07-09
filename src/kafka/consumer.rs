use crate::sql::postgres::PostgresClient;
use crate::sql::schema::DriftRecord;
use anyhow::*;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;

use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::CommitMode;
use rdkafka::Message;

use std::result::Result::Ok;
use tracing::error;
use tracing::info;

// Get table name constant

pub enum MessageHandler {
    Postgres(PostgresClient),
}

impl MessageHandler {
    pub async fn insert_drift_record(&self, record: &DriftRecord) -> Result<()> {
        match self {
            Self::Postgres(client) => {
                let result = client.insert_drift_record(record).await;
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

#[allow(clippy::unnecessary_unwrap)]
#[allow(clippy::too_many_arguments)]
pub async fn setup_kafka_consumer(
    message_handler: MessageHandler,
    group_id: String,
    brokers: String,
    topics: Vec<String>,
    username: Option<String>,
    password: Option<String>,
    security_protocol: Option<String>,
    sasl_mechanism: Option<String>,
) {
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

    loop {
        let message = consumer.recv().await;

        match message {
            Ok(message) => {
                let payload = message.payload_view::<str>().unwrap().unwrap();
                let record: DriftRecord = serde_json::from_str(payload).unwrap();
                message_handler.insert_drift_record(&record).await.unwrap();
                consumer
                    .commit_message(&message, CommitMode::Async)
                    .unwrap();
            }
            Err(e) => {
                info!("Failed to receive message: {:?}", e);
                error!("Failed to receive message: {:?}", e);
            }
        }
    }
}
