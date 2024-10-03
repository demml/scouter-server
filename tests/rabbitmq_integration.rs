use crate::test_utils::*;
use scouter_server::sql::postgres::PostgresClient;
mod test_utils;

#[cfg(feature = "rabbitmq")]
use scouter_server::consumer::rabbitmq::startup::rabbitmq_startup::startup_rabbitmq;

#[cfg(feature = "rabbitmq")]
#[tokio::test]
#[ignore]
async fn test_api_with_rabbitmq() {
    // setup resources
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    let startup = startup_rabbitmq(pool.clone());

    match startup.await {
        Ok(_) => println!("Successfully started rabbitmq consumer"),
        Err(e) => println!("Error starting rabbitmq consumer: {:?}", e),
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // populate rabbit queue (15 messages)
    populate_rabbit_queue().await.unwrap();

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
