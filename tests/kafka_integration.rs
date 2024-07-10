use crate::test_utils::*;

use scouter_server::kafka::consumer::MessageHandler;
use scouter_server::sql::postgres::PostgresClient;
use sqlx::Row;
mod test_utils;
use scouter_server::sql::schema::DriftRecord;

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_api_with_kafka() {
    // setup resources
    let topic_name = "scouter_monitoring";
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    // populate kafka topic (15 messages)
    populate_topic(topic_name).await;

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

    // iterate over the result and create DriftRecord
    for row in result {
        let record = DriftRecord {
            created_at: row.get("created_at"),
            name: row.get("name"),
            repository: row.get("repository"),
            feature: row.get("feature"),
            value: row.get("value"),
            version: row.get("version"),
        };

        println!("{:?}", record);
    }

    assert_eq!(count, 1);
}
