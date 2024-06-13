use crate::sql::postgres::PostgresClient;
use crate::sql::schema::DriftRecord;
use anyhow::*;
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage, MessageSets};
use std::result::Result::Ok;
use tracing::{error, info};

// Get table name constant

pub struct ScouterConsumer {
    pub consumer: Consumer,
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

    pub async fn get_messages(&mut self) -> Result<MessageSets, anyhow::Error> {
        let consumer_poll = match self.consumer.poll() {
            Ok(consumer_poll) => consumer_poll,
            Err(e) => {
                error!("Failed to poll consumer: {:?}", e);
                return Err(e.into());
            }
        };

        Ok(consumer_poll)
    }

    pub async fn insert_messages(
        &mut self,
        messages: MessageSets,
        db_client: &PostgresClient,
    ) -> Result<(), anyhow::Error> {
        if messages.is_empty() {
            info!("No messages to insert");
            return Ok(());
        }

        for ms in messages.iter() {
            for m in ms.messages() {
                // deserialize the message data to DriftRecord
                let drift_record: DriftRecord = match serde_json::from_slice(m.value) {
                    Ok(record) => record,
                    Err(e) => {
                        error!("Failed to deserialize message: {:?}", e);
                        return Err(e.into());
                    }
                };

                let query_result = db_client.insert_drift_record(drift_record).await;

                match query_result {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Failed to insert record into database: {:?}", e);
                        return Err(e.into());
                    }
                }
            }
            match self.consumer.consume_messageset(ms) {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to consume messageset: {:?}", e);
                    return Err(e.into());
                }
            }
        }
        match self.consumer.commit_consumed() {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to commit consumed messages: {:?}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn poll_current(&mut self, db_client: &PostgresClient) -> Result<(), anyhow::Error> {
        // get messages
        let messages = self
            .get_messages()
            .await
            .with_context(|| "Failed to get messages")?;

        // insert messages
        self.insert_messages(messages, db_client)
            .await
            .with_context(|| "Failed to insert messages")?;
        Ok(())
    }

    // Runs indefinite loop that consumes messages from Kafka and inserts them into the database
    //
    // # Arguments
    //
    // * `pool` - A connection pool to the Postgres database
    //
    pub async fn poll_loop(&mut self, db_client: &PostgresClient) -> Result<(), Error> {
        loop {
            match self.poll_current(db_client).await {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to poll current: {:?}", e);
                    continue;
                }
            }
        }
    }
}
