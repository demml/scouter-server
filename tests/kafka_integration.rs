use crate::test_utils::*;

use scouter_server::sql::postgres::PostgresClient;
mod test_utils;

#[cfg(feature = "kafka")]
use scouter_server::consumer::kafka::startup::kafka_startup::startup_kafka;

#[cfg(feature = "kafka")]
#[tokio::test]
#[ignore]
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

    tokio::time::sleep(tokio::time::Duration::from_secs(7)).await;

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
                    LIMIT 10
                    "#,
        )
        .await
        .unwrap();

    let count = result.len();

    assert_eq!(count, 10);
}
