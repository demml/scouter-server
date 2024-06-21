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

pub struct ConsumerConfig {
    pub brokers: String,
    pub topics: Vec<String>,
    pub group: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub security_protocol: Option<String>,
    pub sasl_mechanism: Option<String>,
}

impl ConsumerConfig {
    pub fn new(
        brokers: Option<String>,
        topics: Option<Vec<String>>,
        group: Option<String>,
        username: Option<String>,
        password: Option<String>,
        security_protocol: Option<String>,
        sasl_mechanism: Option<String>,
    ) -> Self {
        let brokers = brokers.unwrap_or_else(|| {
            std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string())
        });

        let topics = topics.unwrap_or_else(|| {
            std::env::var("KAFKA_TOPICS")
                .unwrap_or_else(|_| "scouter_monitoring".to_string())
                .split(",")
                .map(|s| s.to_string())
                .collect()
        });

        let group = group.unwrap_or_else(|| {
            std::env::var("KAFKA_GROUP").unwrap_or_else(|_| "scouter".to_string())
        });

        if username.is_some() || std::env::var("KAFKA_USERNAME").is_ok() {
            let username = Some(username.unwrap_or_else(|| {
                std::env::var("KAFKA_USERNAME")
                    .with_context(|| "KAFKA_USERNAME must be set")
                    .unwrap()
            }));

            let password = Some(password.unwrap_or_else(|| {
                std::env::var("KAFKA_PASSWORD")
                    .with_context(|| "KAFKA_PASSWORD must be set")
                    .unwrap()
            }));

            let security_protocol = Some(security_protocol.unwrap_or_else(|| {
                std::env::var("KAFKA_SECURITY_PROTOCOL").unwrap_or_else(|_| "SASL_SSL".to_string())
            }));

            let sasl_mechanism = Some(sasl_mechanism.unwrap_or_else(|| {
                std::env::var("KAFKA_SASL_MECHANISM").unwrap_or_else(|_| "PLAIN".to_string())
            }));

            return Self {
                brokers,
                topics,
                group,
                username,
                password,
                security_protocol,
                sasl_mechanism,
            };
        } else {
            return Self {
                brokers,
                topics,
                group,
                username: None,
                password: None,
                security_protocol: None,
                sasl_mechanism: None,
            };
        }
    }

    fn to_kafka_config(&self) -> ClientConfig {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", &self.brokers);
        config.set("group.id", &self.group);
        config.set("enable.auto.commit", "false");
        config.set("enable.partition.eof", "false");

        if self.username.is_some() {
            config.set("sasl.username", self.username.clone().unwrap());
        }

        if self.password.is_some() {
            config.set("sasl.password", self.password.clone().unwrap());
        }

        if self.security_protocol.is_some() {
            config.set("security.protocol", self.security_protocol.clone().unwrap());
        }

        if self.sasl_mechanism.is_some() {
            config.set("sasl.mechanisms", self.sasl_mechanism.clone().unwrap());
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
    pub async fn new(
        message_handler: MessageHandler,
        brokers: String,
        topics: Vec<String>,
        group: String,
    ) -> Result<Self, Error> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group)
            .set("enable.auto.commit", "false")
            .set("enable.partition.eof", "false")
            .create()
            .expect("Consumer creation failed");

        let topics = topics.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        consumer
            .subscribe(&topics)
            .expect("Failed to subscribe to topics");

        Ok(Self {
            consumer,
            message_handler,
        })
    }

    pub async fn poll_message(&mut self) -> Result<DriftRecord> {
        let message = self.consumer.recv().await;

        match message {
            Ok(message) => {
                let payload = message.payload_view::<str>().unwrap().unwrap();

                let record: DriftRecord = serde_json::from_str(payload).unwrap();
                self.message_handler.insert_drift_record(&record).await?;

                self.consumer.commit_message(&message, CommitMode::Async)?;
                Ok(record)
            }
            Err(e) => {
                error!("Failed to receive message: {:?}", e);
                Err(e.into())
            }
        }
    }

    pub async fn poll_messages(&mut self) {
        loop {
            let result = self.poll_message().await;
            match result {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to poll message: {:?}", e);
                    continue;
                }
            }
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
        let config = ConsumerConfig::new(None, None, None, None, None, None, None);
        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.topics, vec!["scouter_monitoring"]);
        assert_eq!(config.group, "scouter");
    }

    //#[tokio::test]
    //async fn test_consumer_config_with_auth() {
    //    std::env::set_var("KAFKA_USERNAME", "user");
    //    std::env::set_var("KAFKA_PASSWORD", "password");
    //    std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_SSL");
    //    std::env::set_var("KAFKA_SASL_MECHANISM", "PLAIN");
    //
    //    let config = ConsumerConfig::new(None, None, None, None, None, None, None);
    //    assert_eq!(config.brokers, "localhost:9092");
    //    assert_eq!(config.topics, vec!["scouter_monitoring"]);
    //    assert_eq!(config.group, "scouter");
    //    assert_eq!(config.username, Some("user".to_string()));
    //    assert_eq!(config.password, Some("password".to_string()));
    //    assert_eq!(config.security_protocol, Some("SASL_SSL".to_string()));
    //    assert_eq!(config.sasl_mechanism, Some("PLAIN".to_string()));
    //}
    //
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

        //let consumer: StreamConsumer = ClientConfig::new()
        //    .set("bootstrap.servers", cluster.bootstrap_servers())
        //    .set("group.id", "rust-rdkafka-mock-example")
        //    .create()
        //    .expect("Consumer creation failed");
        //consumer.subscribe(&[topic]).unwrap();

        let mut consumer = ScouterConsumer::new(
            MessageHandler::Mock(MockInserter),
            cluster.bootstrap_servers(),
            vec![topic.to_string()],
            "rust-rdkafka-mock-example".to_string(),
        )
        .await
        .unwrap();

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

        //let message = scouter_consumer.poll_message().await.unwrap();

        let message = consumer.poll_message().await.unwrap();
        // convert message to string
        //let message_str = message.payload_view::<str>().unwrap().unwrap();

        println!("Received message: {:?}", message);

        //assert_eq!(message_str, "hello");
    }
}
