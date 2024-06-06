use crate::postgres::PostgresClient;
use crate::schema::DriftRecord;
use anyhow::*;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use sqlx::{Pool, Postgres};
use std::result::Result::Ok;
use std::time::Duration;
use tracing::{error, info};

// Get table name constant

pub struct ScouterConsumer {
    consumer: Consumer,
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
    pub fn new() -> Result<Self, Error> {
        let brokers = std::env::var("KAFKA_BROKER").with_context(|| "KAFKA_BROKER must be set")?;
        let brokers = brokers
            .split(",")
            .map(|broker| broker.to_string())
            .collect::<Vec<String>>();

        let kafka_topic =
            std::env::var("KAFKA_TOPIC").unwrap_or_else(|_| "scouter_monitoring".to_string());
        let kafka_group = std::env::var("KAFKA_GROUP").unwrap_or_else(|_| "scouter".to_string());
        let kafka_partitions: Option<Vec<i32>> =
            std::env::var("KAFKA_PARTITIONS").ok().map(|partitions| {
                partitions
                    .split(",")
                    .map(|partition| partition.parse::<i32>().unwrap())
                    .collect::<Vec<i32>>()
                    .try_into()
                    .unwrap()
            });

        let consumer = Consumer::from_hosts(brokers);

        let consumer = match kafka_partitions {
            Some(partitions) => consumer.with_topic_partitions(kafka_topic, &partitions),
            None => consumer.with_topic(kafka_topic),
        };

        let consumer = consumer
            .with_group(kafka_group)
            .with_fallback_offset(FetchOffset::Earliest)
            .with_offset_storage(Some(GroupOffsetStorage::Kafka))
            .create()
            .context("Failed to create Kafka consumer")?;

        Ok(Self { consumer })
    }

    // Runs indefinite loop that consumes messages from Kafka and inserts them into the database
    //
    // # Arguments
    //
    // * `pool` - A connection pool to the Postgres database
    //
    pub async fn poll(&mut self, postgres_client: &PostgresClient) -> Result<(), Error> {
        let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "drift".to_string());

        loop {
            // check consumer poll
            let consumer_poll = match self.consumer.poll() {
                Ok(consumer_poll) => consumer_poll,
                Err(e) => {
                    error!("Failed to poll consumer: {:?}", e);
                    continue;
                }
            };

            // start polling for messages
            for ms in consumer_poll.iter() {
                for m in ms.messages() {
                    // deserialize the message data to DriftRecord
                    let drift_record: DriftRecord = match serde_json::from_slice(m.value) {
                        Ok(record) => record,
                        Err(e) => {
                            error!("Failed to deserialize message: {:?}", e);
                            continue;
                        }
                    };

                    let query_result = postgres_client
                        .insert_drift_record(drift_record, &table_name)
                        .await;

                    match query_result {
                        Ok(_) => (),
                        Err(e) => error!("Failed to insert record into database: {:?}", e),
                    }
                }
                match self.consumer.consume_messageset(ms) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Failed to consume messageset: {:?}", e);
                        continue;
                    }
                }
            }
            match self.consumer.commit_consumed() {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to commit consumed messages: {:?}", e);
                    continue;
                }
            }
        }
    }
}

// tests
#[cfg(test)]
mod tests {

    use super::*;
    use kafka::producer::{Producer, Record, RequiredAcks};
    use std::env;

    #[test]
    fn test_scouter_consumer() {
        env::set_var("KAFKA_BROKER", "localhost:9092");

        // populate the kafka topic
        let mut producer = Producer::from_hosts(vec!["localhost:9092".to_owned()])
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();

        let mut buf = String::with_capacity(10);
        for i in 0..10 {
            let record = DriftRecord {
                service_name: "test".to_string(),
                feature: "test".to_string(),
                value: i as f64,
                version: "test".to_string(),
            };
            producer
                .send(&Record::from_value(
                    "scouter_monitoring",
                    serde_json::to_string(&record).unwrap().as_bytes(),
                ))
                .unwrap();
            buf.clear();

            let scouter_consumer = ScouterConsumer::new().unwrap();

            // poll the consumer
        }
    }
}
