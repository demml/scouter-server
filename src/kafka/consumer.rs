use crate::sql::postgres::PostgresClient;
use crate::sql::schema::DriftRecord;
use anyhow::*;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::Consumer;

use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::CommitMode;
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use sqlx::any;
use sqlx::postgres::{PgPoolOptions, PgQueryResult};

use std::result::Result::Ok;
use tracing::error;

// Get table name constant

struct MockInserter;

impl MockInserter {
    fn insert_drift_record(&self, record: &DriftRecord) -> Result<()> {
        println!("Mock insert drift record: {:?}", record);
        Ok(())
    }
}

pub enum MessageHandler {
    Postgres(PostgresClient),
    Mock(MockInserter),
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
                        ()
                    }
                }
            }
            Self::Mock(mock) => {
                let result = mock.insert_drift_record(record);
                match result {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Failed to insert drift record: {:?}", e);
                        ()
                    }
                }
            }
        }

        Ok(())
    }
}

struct ConsumerConfig {
    pub brokers: String,
    pub topics: Vec<String>,
    pub group: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub security_protocol: Option<String>,
    pub sasl_mechanism: Option<String>,
}

impl ConsumerConfig {
    pub fn new() -> Self {
        let brokers =
            std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
        let topics = std::env::var("KAFKA_TOPICS")
            .unwrap_or_else(|_| "scouter_monitoring".to_string())
            .split(",")
            .map(|topic| topic.to_string())
            .collect::<Vec<String>>();
        let group = std::env::var("KAFKA_GROUP").unwrap_or_else(|_| "scouter".to_string());

        let mut username = std::env::var("KAFKA_USERNAME").ok();

        let mut password = None;
        let mut security_protocol = None;
        let mut sasl_mechanism = None;

        if username.is_some() {
            password = Some(
                std::env::var("KAFKA_PASSWORD")
                    .with_context(|| "KAFKA_PASSWORD must be set")
                    .unwrap(),
            );
            security_protocol = Some(
                std::env::var("KAFKA_SECURITY_PROTOCOL").unwrap_or_else(|_| "SASL_SSL".to_string()),
            );
            sasl_mechanism =
                Some(std::env::var("KAFKA_SASL_MECHANISM").unwrap_or_else(|_| "PLAIN".to_string()));
        }

        Self {
            brokers,
            topics,
            group,
            username,
            password,
            security_protocol,
            sasl_mechanism,
        }
    }

    fn to_kafka_config(&self) -> ClientConfig {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", &self.brokers);
        config.set("group.id", &self.group);
        config.set("enable.auto.commit", "false");
        config.set("enable.partition.eof", "false");

        if let Some(username) = &self.username {
            config.set("sasl.username", username);
        }

        if let Some(password) = &self.password {
            config.set("sasl.password", password);
        }

        if let Some(security_protocol) = &self.security_protocol {
            config.set("security.protocol", security_protocol);
        }

        if let Some(sasl_mechanism) = &self.sasl_mechanism {
            config.set("sasl.mechanisms", sasl_mechanism);
        }

        config
    }
}

pub struct ScouterConsumer {
    pub consumer: StreamConsumer,
    pub message_handler: MessageHandler,
}

impl ScouterConsumer {
    // Create a new instance of ScouterConsumer
    // Things that happen on initialization
    // 1. Get the Kafka broker address from the environment variable KAFKA_BROKER
    // 2. Get the Kafka topic from the environment variable KAFKA_TOPIC
    // 3. Get the Kafka group from the environment variable KAFKA_GROUP
    // 4. Get the Kafka partitions from the environment variable KAFKA_PARTITIONS
    // 5. Create a new Kafka consumer with the above configurations
    //
    // # Returns
    //
    // A Result containing the ScouterConsumer instance or an Error
    pub async fn new(message_handler: MessageHandler) -> Result<Self, Error> {
        let consumer_config = ConsumerConfig::new();
        let mut config = consumer_config.to_kafka_config();

        config.set_log_level(RDKafkaLogLevel::Info);
        let consumer: StreamConsumer = config
            .create()
            .with_context(|| "Failed to create Kafka consumer")?;

        let topics = consumer_config
            .topics
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();

        consumer
            .subscribe(&topics)
            .with_context(|| "Failed to subscribe to Kafka topics")?;

        Ok(Self {
            consumer,
            message_handler,
        })
    }

    pub async fn poll_message(&mut self) -> Result<()> {
        let consumed = match self.consumer.recv().await {
            Err(e) => {
                error!("Failed to poll consumer: {:?}", e);
                ()
            }
            Ok(message) => {
                // map to DriftRecord
                let record: Option<DriftRecord> = if message.payload().is_none() {
                    None
                } else {
                    match message.payload_view::<str>() {
                        Some(Ok(payload)) => match serde_json::from_str(payload) {
                            Ok(record) => Some(record),
                            Err(e) => {
                                error!("Failed to deserialize message: {:?}", e);
                                None
                            }
                        },
                        Some(Err(e)) => {
                            error!("Failed to get payload view: {:?}", e);
                            None
                        }
                        None => None,
                    }
                };

                if record.is_some() {
                    self.message_handler
                        .insert_drift_record(&record.unwrap())
                        .await?;
                }

                self.consumer.commit_message(&message, CommitMode::Async)?;
            }
        };

        Ok(consumed)
    }

    pub async fn poll_messages(&mut self) {
        loop {
            self.poll_message().await.unwrap();
        }
    }
}

// test kafka

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::mocking::*;
    use rdkafka::producer::{FutureProducer, FutureRecord};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn test_consumer_config() {
        let config = ConsumerConfig::new();
        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.topics, vec!["scouter_monitoring"]);
        assert_eq!(config.group, "scouter");
    }

    #[tokio::test]
    async fn test_consumer_config_with_auth() {
        std::env::set_var("KAFKA_USERNAME", "user");
        std::env::set_var("KAFKA_PASSWORD", "password");
        std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_SSL");
        std::env::set_var("KAFKA_SASL_MECHANISM", "PLAIN");

        let config = ConsumerConfig::new();
        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.topics, vec!["scouter_monitoring"]);
        assert_eq!(config.group, "scouter");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("password".to_string()));
        assert_eq!(config.security_protocol, Some("SASL_SSL".to_string()));
        assert_eq!(config.sasl_mechanism, Some("PLAIN".to_string()));
    }

    #[tokio::test]
    async fn test_kafka_consumer() {
        let cluster = MockCluster::new(1).unwrap();
        let topic = "scouter_monitoring";
        cluster.create_topic(topic, 1, 1).unwrap();

        let message =
            r#"{"name": "test", "version": "1.0.0", "max_data_points": 100, "time_window": "1h"}"#;

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", cluster.bootstrap_servers())
            .create()
            .expect("Producer creation error");

        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", cluster.bootstrap_servers())
            .set("group.id", "rust-rdkafka-mock-example")
            .create()
            .expect("Consumer creation failed");
        consumer.subscribe(&[topic]).unwrap();

        let record = DriftRecord {
            created_at: chrono::Utc::now().naive_utc(),
            name: "test".to_string(),
            repository: "test".to_string(),
            feature: "test".to_string(),
            value: 0.0,
            version: "1.0.0".to_string(),
        };

        let record_string = serde_json::to_string(&record).unwrap();

        tokio::spawn(async move {
            let mut i = 0_usize;
            loop {
                producer
                    .send_result(
                        FutureRecord::to(topic)
                            .key(&i.to_string())
                            .payload(&record_string),
                    )
                    .unwrap()
                    .await
                    .unwrap()
                    .unwrap();
                i += 1;
            }
        });

        let start = Instant::now();
        println!("Warming up for 10s...");
        // warm up for 10s
        while start.elapsed() < Duration::from_secs(5) {
            continue;
        }

        let message = consumer.recv().await.unwrap();
        // convert message to string
        let message_str = message.payload_view::<str>().unwrap().unwrap();

        println!("Received message: {:?}", message);
        assert_eq!(message_str, "hello");
    }
}
