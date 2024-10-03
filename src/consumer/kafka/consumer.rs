#[cfg(feature = "kafka")]
pub mod kafka_consumer {
    use crate::consumer::base::MessageHandler;

    use anyhow::*;
    use rdkafka::config::ClientConfig;
    use rdkafka::consumer::Consumer;
    use scouter::core::drift::base::ServerRecords;

    use futures::StreamExt;
    use rdkafka::consumer::CommitMode;
    use rdkafka::consumer::StreamConsumer;
    use rdkafka::message::BorrowedMessage;
    use rdkafka::message::Message;
    use std::collections::HashMap;
    use std::result::Result::Ok;
    use tracing::error;
    use tracing::info;

    // Get table name constant

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::unnecessary_unwrap)]
    pub async fn create_kafka_consumer(
        group_id: String,
        brokers: String,
        topics: Vec<String>,
        username: Option<String>,
        password: Option<String>,
        security_protocol: Option<String>,
        sasl_mechanism: Option<String>,
        config_overrides: Option<HashMap<&str, &str>>,
    ) -> Result<StreamConsumer, anyhow::Error> {
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

        if let Some(overrides) = config_overrides {
            for (key, value) in overrides {
                config.set(key, value);
            }
        }

        let consumer: StreamConsumer = config.create().expect("Consumer creation error");

        let topics = topics.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

        consumer
            .subscribe(&topics)
            .expect("Can't subscribe to specified topics");

        info!("✅ Started consumer for topics: {:?}", topics);
        Ok(consumer)
    }

    pub async fn stream_from_kafka_topic(
        message_handler: &MessageHandler,
        consumer: &StreamConsumer,
    ) -> Result<()> {
        // start stream
        let mut stream = consumer.stream();

        // get next message. If no message is received, log error
        if let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    // extract payload from message. If no payload is received, log error
                    if let Some(payload) = extract_payload(&msg) {
                        // deserialize payload. If deserialization fails, log error
                        if let Some(records) = deserialize_payload(payload) {
                            if let Err(e) =
                                insert_and_commit(message_handler, consumer, &msg, &records).await
                            {
                                error!("Failed to insert and commit: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => error!("Failed to receive message: {:?}", e),
            }
        } else {
            error!("No message received");
        }

        Ok(())
    }

    fn extract_payload<'a>(msg: &'a BorrowedMessage<'a>) -> Option<&'a [u8]> {
        match msg.payload() {
            Some(payload) => Some(payload),
            None => {
                error!("No payload received");
                None
            }
        }
    }

    fn deserialize_payload(payload: &[u8]) -> Option<ServerRecords> {
        match serde_json::from_slice(payload) {
            Ok(records) => Some(records),
            Err(e) => {
                error!("Failed to deserialize message: {:?}", e);
                None
            }
        }
    }

    async fn insert_and_commit<'a>(
        message_handler: &MessageHandler,
        consumer: &StreamConsumer,
        msg: &'a BorrowedMessage<'a>,
        records: &ServerRecords,
    ) -> Result<()> {
        match message_handler.insert_server_records(records).await {
            Ok(_) => {
                consumer.commit_message(msg, CommitMode::Async).unwrap();
                Ok(())
            }
            Err(e) => {
                error!("Failed to insert drift record: {:?}", e);
                Ok(())
            }
        }
    }

    // Start background task to poll kafka topic
    //
    // This function will poll the kafka topic and insert the records into the database
    // using the provided message handler.
    //
    // # Arguments
    //
    // * `message_handler` - The message handler to process the records
    // * `group_id` - The kafka consumer group id
    // * `brokers` - The kafka brokers
    // * `topics` - The kafka topics to subscribe to
    // * `username` - The kafka username
    // * `password` - The kafka password
    // * `security_protocol` - The kafka security protocol
    // * `sasl_mechanism` - The kafka SASL mechanism
    //
    // # Returns
    //
    // * `Result<(), anyhow::Error>` - The result of the operation
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
            None,
        )
        .await
        .unwrap();

        loop {
            stream_from_kafka_topic(&message_handler, &consumer).await?;
        }
    }
}
