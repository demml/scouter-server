use crate::sql::postgres::PostgresClient;
use crate::sql::schema::DriftRecord;
use anyhow::*;

use futures::StreamExt;

use std::collections::HashMap;
use std::result::Result::Ok;
use tracing::error;
use tracing::info;

use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties, Consumer, Result,
};

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

pub async fn create_rabbitmq_consumer(address: &str) -> Result<Consumer> {
    let conn = Connection::connect(address, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await.unwrap();

    channel
        .queue_declare(
            "scouter_monitoring",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let consumer = channel
        .basic_consume(
            "scouter_monitoring",
            "scouter_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("✅ Started consumer for RabbitMQ");
    Ok(consumer)
}

pub async fn stream_from_rabbit_queue(
    message_handler: &MessageHandler,
    consumer: &mut Consumer,
) -> Result<()> {
    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let message = std::str::from_utf8(&delivery.data).unwrap();
            info!("Received message: {}", message);

            // Acknowledge the message
            delivery.ack(BasicAckOptions::default()).await?;
        }
    }

    Ok(())
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

pub async fn start_rabbitmq_background_poll(
    message_handler: MessageHandler,
    address: String,
) -> Result<()> {
    let mut consumer = create_rabbitmq_consumer(&address).await?;

    loop {
        stream_from_rabbit_queue(&message_handler, &mut consumer).await?;
    }
}
