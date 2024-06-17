use kafka::producer::Record;
use scouter_server::sql::schema::DriftRecord;
use tokio;
mod common;

#[tokio::test]
async fn test_scouter_consumer() {
    let (db_client, mut scouter_consumer, mut producer) = common::setup().await.unwrap();

    // check there are no messages that have been pushed to the db
    let results = db_client
        .raw_query(
            r#"
        SELECT * 
        FROM scouter.drift  
        WHERE service_name = 'test_app'
        "#,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 0);

    for i in 0..10 {
        let record = DriftRecord {
            created_at: Some(chrono::Utc::now().naive_utc()),
            service_name: "test_app".to_string(),
            feature: "test".to_string(),
            value: i as f64,
            version: "1.0.0".to_string(),
        };
        producer
            .send(&Record::from_value(
                "scouter_monitoring",
                serde_json::to_string(&record).unwrap().as_bytes(),
            ))
            .unwrap();
    }

    // poll the consumer
    scouter_consumer.poll_current(&db_client).await.unwrap();

    let results = db_client
        .raw_query(
            r#"
        SELECT * 
        FROM scouter.drift  
        WHERE service_name = 'test_app'
        "#,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 10);

    // check there are no more messages
    let next_msgs = scouter_consumer.get_messages().await.unwrap();

    // assert no more messages
    assert!(next_msgs.is_empty());

    common::teardown(&db_client).await.unwrap();
}
