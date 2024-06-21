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

// Get table name constant

pub struct MockInserter;

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
        username: Option<String>,
        password: Option<String>,
        security_protocol: Option<String>,
        sasl_mechanism: Option<String>,
    ) -> Result<Self, Error> {
        let mut config = ClientConfig::new();

        config
            .set("bootstrap.servers", brokers)
            .set("group.id", group)
            .set("enable.auto.commit", "false")
            .set("enable.partition.eof", "false");

        if username.is_some() {
            config.set("sasl.username", username.clone().unwrap());
        }

        if password.is_some() {
            config.set("sasl.password", password.clone().unwrap());
        }

        if security_protocol.is_some() {
            config.set("security.protocol", security_protocol.clone().unwrap());
        }

        if sasl_mechanism.is_some() {
            config.set("sasl.mechanisms", sasl_mechanism.clone().unwrap());
        }

        let consumer: StreamConsumer = config
            .create()
            .with_context(|| "Failed to create consumer")?;

        let topics = topics.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        consumer
            .subscribe(&topics)
            .expect("Failed to subscribe to topics");

        Ok(Self {
            consumer,
            message_handler,
        })
    }

    pub async fn poll_message(&mut self) -> Result<Option<DriftRecord>> {
        println!("Polling message");
        let message = self.consumer.recv().await;

        match message {
            Ok(message) => {
                let payload = message.payload_view::<str>().unwrap().unwrap();

                let record: DriftRecord = serde_json::from_str(payload).unwrap();
                self.message_handler.insert_drift_record(&record).await?;

                self.consumer.commit_message(&message, CommitMode::Async)?;

                Ok(Some(record))
            }
            Err(e) => {
                error!("Failed to receive message: {:?}", e);
                Ok(None)
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
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn test_kafka_consumer() {
        let cluster = MockCluster::new(1).unwrap();
        let topic = "scouter_monitoring";
        cluster.create_topic(topic, 1, 1).unwrap();

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", cluster.bootstrap_servers())
            .create()
            .expect("Producer creation error");

        let mut consumer = ScouterConsumer::new(
            MessageHandler::Mock(MockInserter),
            cluster.bootstrap_servers(),
            vec![topic.to_string()],
            "scouter".to_string(),
            None,
            None,
            None,
            None,
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
        // warm up for 10s
        while start.elapsed() < Duration::from_secs(5) {
            continue;
        }

        let message = consumer.poll_message().await.unwrap().unwrap();

        assert_eq!(message.name, "test");
        assert_eq!(message.repository, "test");
        assert_eq!(message.feature, "test");
        assert_eq!(message.value, 0.0);
    }
}
