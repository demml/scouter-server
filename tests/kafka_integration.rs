use crate::test_utils::*;
use scouter_server::kafka::consumer::{
    create_kafka_consumer, start_kafka_background_poll, stream_from_kafka_topic, MessageHandler,
};
use scouter_server::sql::postgres::PostgresClient;
use sqlx::Row;
mod test_utils;
use scouter_server::sql::schema::DriftRecord;

#[tokio::test(flavor = "multi_thread")]
async fn test_api_with_kafka() {
    // setup resources
    let topic_name = "scouter_monitoring";
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();
    let brokers = std::env::var("KAFKA_BROKERS").unwrap();
    let topics = vec![std::env::var("KAFKA_TOPIC").unwrap_or("scouter_monitoring".to_string())];
    let group_id = std::env::var("KAFKA_GROUP").unwrap_or("scouter".to_string());
    let username: Option<String> = std::env::var("KAFKA_USERNAME").ok();
    let password: Option<String> = std::env::var("KAFKA_PASSWORD").ok();
    let security_protocol: Option<String> = Some(
        std::env::var("KAFKA_SECURITY_PROTOCOL")
            .ok()
            .unwrap_or_else(|| "SASL_SSL".to_string()),
    );
    let sasl_mechanism: Option<String> = Some(
        std::env::var("KAFKA_SASL_MECHANISM")
            .ok()
            .unwrap_or_else(|| "PLAIN".to_string()),
    );

    // populate kafka topic (15 messages)
    let test = populate_topic(topic_name);
    match test.await {
        Ok(_) => println!("Successfully populated kafka topic"),
        Err(e) => println!("Error populating kafka topic: {:?}", e),
    }

    // start background task to poll kafka topic
    tokio::task::spawn(async move {
        let message_handler = MessageHandler::Postgres(db_client.clone());
        start_kafka_background_poll(
            message_handler,
            group_id,
            brokers,
            topics,
            username,
            password,
            security_protocol,
            sasl_mechanism,
        )
    });

    // sleep for 5 seconds to allow kafka to process messages
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    //let result = db_client
    //    .raw_query(
    //        r#"
    //                SELECT *
    //                FROM scouter.drift
    //                WHERE name = 'test_app'
    //                LIMIT 10
    //                "#,
    //    )
    //    .await
    //    .unwrap();
    //
    //let count = result.len();
    //
    //// iterate over the result and create DriftRecord
    //for row in result {
    //    let record = DriftRecord {
    //        created_at: row.get("created_at"),
    //        name: row.get("name"),
    //        repository: row.get("repository"),
    //        feature: row.get("feature"),
    //        value: row.get("value"),
    //        version: row.get("version"),
    //    };
    //
    //    println!("{:?}", record);
    //}
    //
    //assert_eq!(count, 1);
}
