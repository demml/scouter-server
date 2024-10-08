use sqlx::Row;

use scouter_server::api::schema::{DriftAlertRequest, DriftRequest};
use scouter_server::sql::postgres::PostgresClient;
mod test_utils;
use scouter::core::drift::spc::types::SpcServerRecord;
use scouter_server::api::schema::ServiceInfo;
use std::collections::BTreeMap;

#[tokio::test]
async fn test_postgres_client() {
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    // test inserting record
    let record = SpcServerRecord {
        created_at: chrono::Utc::now().naive_utc(),
        name: "test_app".to_string(),
        repository: "test".to_string(),
        feature: "test".to_string(),
        value: 1.0,
        version: "1.0.0".to_string(),
    };

    db_client.insert_spc_drift_record(&record).await.unwrap();

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
        let record = SpcServerRecord {
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
    let service_info = ServiceInfo {
        name: "test_app".to_string(),
        repository: "test".to_string(),
        version: "1.0.0".to_string(),
    };

    let result = db_client
        .get_drift_records(&service_info, limit_timestamp.to_string().as_str(), &[])
        .await
        .unwrap();

    assert_eq!(result.features.len(), 1);

    // send feature alerts
    let mut alerts = BTreeMap::new();
    alerts.insert("zone".to_string(), "test".to_string());

    for i in 0..3 {
        let feature_name = format!("test_feature_{}", i);
        db_client
            .insert_drift_alert(&service_info, &feature_name, &alerts)
            .await
            .unwrap();
        // sleep for 1 second
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // get alerts
    let drift_alert_request = DriftAlertRequest {
        name: record.name.clone(),
        repository: record.repository.clone(),
        version: record.version.clone(),
        limit_timestamp: None,
        active: Some(true),
        limit: None,
    };
    let result = db_client
        .get_drift_alerts(&drift_alert_request)
        .await
        .unwrap();

    assert_eq!(result.len(), 3);

    let drift_alert_request = DriftAlertRequest {
        name: record.name.clone(),
        repository: record.repository.clone(),
        version: record.version.clone(),
        limit_timestamp: Some(result[0].created_at.to_string()),
        active: Some(true),
        limit: Some(50),
    };
    let result = db_client
        .get_drift_alerts(&drift_alert_request)
        .await
        .unwrap();

    assert_eq!(result.len(), 1);

    let drift_request = DriftRequest {
        name: record.name.clone(),
        repository: record.repository.clone(),
        version: record.version.clone(),
        time_window: "30minute".to_string(),
        max_data_points: 1000,
    };

    //test get_binned_drift_records
    let result = db_client
        .get_binned_drift_records(&drift_request)
        .await
        .unwrap();

    assert_eq!(result.features.len(), 1);

    test_utils::teardown().await.unwrap();
}
