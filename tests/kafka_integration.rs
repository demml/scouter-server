use crate::test_utils::*;
use anyhow::Context;
use futures::future;
use rdkafka::Message;
use scouter_server::kafka::consumer::{
    create_kafka_consumer, start_kafka_background_poll, stream_from_kafka_topic, MessageHandler,
};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::{BaseConsumer, StreamConsumer};
use rdkafka::producer::FutureProducer;
mod test_utils;

#[tokio::test]
async fn test_produce_consume_base() {
    let topic_name = "scouter_monitoring";
    let pool = test_utils::setup_pool_and_clean_db().await.unwrap();
    let db_client = PostgresClient::new(pool.clone())
        .with_context(|| "Failed to create Postgres client")
        .unwrap();
    let message_handler = MessageHandler::Postgres(db_client);

    populate_topic(topic_name).await;

    let config = test_utils::consumer_config("scouter", None);
    let consumer = create_kafka_consumer(
        "scouter".to_string(),
        "localhost:9092".to_string(),
        ["scouter_monitoring".to_string()].to_vec(),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let _ = stream_from_kafka_topic(&message_handler, &consumer).await;

    //let consumer = create_kafka_consumer(
    //    "scouter".to_string(),
    //    "localhost:9092".to_string(),
    //    ["scouter_monitoring".to_string()].to_vec(),
    //    None,
    //    None,
    //    None,
    //    None,
    //)
    //.await
    //.unwrap();
    //
    //consumer
    //    .stream()
    //    .take(3)
    //    .for_each(|message| async {
    //        let message = message.unwrap();
    //        let payload = message.payload_view::<str>().unwrap().unwrap();
    //        println!("Message payload: {}", payload);
    //    })
    //    .await;
}
