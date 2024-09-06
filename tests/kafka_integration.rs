use crate::test_utils::*;
use scouter_server::kafka::startup::startup_kafka;
use scouter_server::sql::postgres::PostgresClient;
mod test_utils;

#[tokio::test]
async fn test_api_with_kafka() {
    // setup resources
    let topic_name = "scouter_monitoring";
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    let startup = startup_kafka(pool.clone());

    match startup.await {
        Ok(_) => println!("Successfully started kafka"),
        Err(e) => println!("Error starting kafka: {:?}", e),
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // populate kafka topic (15 messages)
    let test = populate_topic(topic_name);
    match test.await {
        Ok(_) => println!("Successfully populated kafka topic"),
        Err(e) => println!("Error populating kafka topic: {:?}", e),
    }

    // sleep for 5 seconds to allow kafka to process messages
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let result = db_client
        .raw_query(
            r#"
                    SELECT *
                    FROM scouter.drift
                    WHERE name = 'test_app'
                    "#,
        )
        .await
        .unwrap();

    let count = result.len();

    assert_eq!(count, 45);
}
