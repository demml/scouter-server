use crate::rabbitmq::consumer::{start_rabbitmq_background_poll, MessageHandler};
use crate::sql::postgres::PostgresClient;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties, Result,
};
use sqlx::{Pool, Postgres};
use tracing::info;

pub async fn startup_rabbitmq(pool: Pool<Postgres>) -> Result<()> {
    info!("Starting RabbitMQ consumer");

    let num_rabbits = std::env::var("NUM_SCOUTER_RABBITMQ_CONSUMERS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<usize>()
        .map_err(|e| lapin::Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    for _ in 0..num_rabbits {
        let rabbit_db_client = PostgresClient::new(pool.clone()).unwrap();
        let message_handler = MessageHandler::Postgres(rabbit_db_client);

        let rabbit_addr = std::env::var("RABBITMQ_ADDR")
            .unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into());

        tokio::spawn(async move {
            start_rabbitmq_background_poll(message_handler, rabbit_addr)
                .await
                .unwrap();
        });
    }

    Ok(())
}
