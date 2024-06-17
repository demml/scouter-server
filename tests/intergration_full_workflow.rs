use tokio;
mod common;
use approx::relative_eq;
use kafka::producer::Record;
use scouter_server::sql::postgres::TimeInterval;
use scouter_server::sql::schema::DriftRecord;

#[tokio::test]
async fn test_full_test() {
    let (db_client, mut scouter_consumer, mut producer) = common::setup().await.unwrap();

    // feature name vec
    let feature_names = vec!["feature1", "feature2", "feature3"];

    // for each features, generate 1000 records
    feature_names.iter().for_each(|feature_name| {
        for i in 0..1000 {
            let record = DriftRecord {
                created_at: Some(chrono::Utc::now().naive_utc()),
                service_name: "test_app".to_string(),
                feature: feature_name.to_string(),
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
    });
    let not_empty = true;

    // poll and insert the messages until the consumer is empty
    while not_empty {
        let messages = scouter_consumer.get_messages().await.unwrap();
        if messages.is_empty() {
            break;
        }
        scouter_consumer
            .insert_messages(messages, &db_client)
            .await
            .unwrap();
    }

    // check the number of records in the database
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

    assert_eq!(results.len(), 3000);

    let max_data_points = 850 as i32;

    // get drift records
    let data = db_client
        .read_drift_records(
            "test_app",
            "1.0.0",
            &max_data_points,
            &TimeInterval::FiveMinutes.to_minutes(),
        )
        .await
        .unwrap();

    // check the number of features
    assert_eq!(data.features.len(), 3);

    // check the number of data points
    for (_, feature_result) in data.features.iter() {
        assert!(relative_eq!(
            feature_result.values.len() as f64,
            3.0 as f64,
            epsilon = 1.0
        ));
    }
}
