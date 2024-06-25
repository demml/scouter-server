use scouter_server::sql::schema::DriftRecord;
use sqlx::Row;
use tokio;
mod common;

#[tokio::test]
async fn test_postgres_client() {
    let db_client = common::setup_db().await.unwrap();

    // test inserting record
    let record = DriftRecord {
        created_at: chrono::Utc::now().naive_utc(),
        name: "test_app".to_string(),
        repository: "test".to_string(),
        feature: "test".to_string(),
        value: 1.0,
        version: "1.0.0".to_string(),
    };

    db_client.insert_drift_record(&record).await.unwrap();

    let result = db_client
        .raw_query(
            r#"
                    SELECT * 
                    FROM scouter.drift  
                    WHERE name = 'test_app'
                    LIMIT 1
                    "#,
        )
        .await
        .unwrap();

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

        assert_eq!(record.name, "test_app");
        assert_eq!(record.repository, "test");
        assert_eq!(record.feature, "test");
        assert_eq!(record.value, 1.0);
        assert_eq!(record.version, "1.0.0");
    }

    // test querying drift records
    // subtract 1 minute from created at
    let limit_timestamp = record.created_at - chrono::Duration::minutes(1);

    let result = db_client
        .get_drift_records(
            "test_app",
            "test",
            "1.0.0",
            limit_timestamp.to_string().as_str(),
        )
        .await
        .unwrap();

    assert_eq!(result.features.len(), 1);

    common::teardown().await.unwrap();
}
