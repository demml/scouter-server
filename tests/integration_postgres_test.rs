use scouter_server::sql::schema::DriftRecord;
use sqlx::Row;
use tokio;
mod common;

#[tokio::test]
async fn test_postgres_client() {
    let (db_client, mut _scouter_consumer, mut _producer) = common::setup().await.unwrap();

    // test inserting record
    let record = DriftRecord {
        service_name: "test_service".to_string(),
        feature: "test".to_string(),
        value: 1.0,
        version: "1.0.0".to_string(),
    };

    db_client.insert_drift_record(record).await.unwrap();

    let result = db_client
        .raw_query(
            r#"
                    SELECT * 
                    FROM scouter.drift  
                    WHERE service_name = 'test_service'
                    LIMIT 1
                    "#,
        )
        .await
        .unwrap();

    // iterate over the result and create DriftRecord
    for row in result {
        let record = DriftRecord {
            service_name: row.get("service_name"),
            feature: row.get("feature"),
            value: row.get("value"),
            version: row.get("version"),
        };

        assert_eq!(record.service_name, "test_service");
        assert_eq!(record.feature, "test");
        assert_eq!(record.value, 1.0);
        assert_eq!(record.version, "1.0.0");
    }

    common::teardown(&db_client).await.unwrap();
}
