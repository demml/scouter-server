use axum::{
    body::Body,
    http::{self, Request, StatusCode},
};
use http_body_util::BodyExt;
use scouter::core::dispatch::types::AlertDispatchType;
use scouter::core::drift::base::DriftType;
use scouter::core::drift::spc::types::{
    SpcAlertConfig, SpcAlertRule, SpcDriftConfig, SpcDriftProfile, SpcFeatureDriftProfile,
};
use scouter_server::api::schema::{DriftRecordRequest, ProfileStatusRequest};
use scouter_server::sql::schema::QueryResult;
use serde_json::Value;
use std::collections::HashMap;
use tower::Service;
use tower::ServiceExt; // for `call`, `oneshot`, and `ready`
mod test_utils;
use scouter_server::alerts::drift::DriftExecutor;
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::AlertResult;
use sqlx::Row;

#[tokio::test]
async fn test_api_drift() {
    let mut app = test_utils::setup_api(true).await.unwrap();

    // create 3 records and insert
    for i in 0..3 {
        let record = DriftRecordRequest {
            created_at: None,
            name: "test_app".to_string(),
            repository: "test".to_string(),
            feature: format!("feature{}", i),
            value: i as f64,
            version: "1.0.0".to_string(),
        };

        let body = serde_json::to_string(&record).unwrap();

        let response = app
            .call(
                Request::builder()
                    .uri("/scouter/drift")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .method("POST")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // query data
    let response = app.call(
        Request::builder()
            .uri("/scouter/drift?name=test_app&repository=test&version=1.0.0&time_window=5minute&max_data_points=1000")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // get data field from response
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();

    let data = body.get("data");
    let data: QueryResult = serde_json::from_value(data.unwrap().clone()).unwrap();

    assert_eq!(data.features.len(), 3);

    let record = DriftRecordRequest {
        created_at: None,
        name: "test_app".to_string(),
        repository: "test".to_string(),
        feature: "feature1".to_string(),
        value: 2.5,
        version: "2.0.0".to_string(),
    };

    let body = serde_json::to_string(&record).unwrap();

    // insert data for new version
    let response = app
        .call(
            Request::builder()
                .uri("/scouter/drift")
                .header(http::header::CONTENT_TYPE, "application/json")
                .method("POST")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // query new version
    let response = app.call(
        Request::builder()
            .uri("/scouter/drift?name=test_app&repository=test&version=2.0.0&time_window=5minute&max_data_points=1000")
            .method("GET")
            .body(Body::empty())
            .unwrap(),
    );

    let response = response.await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // get data field from response
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();

    let data = body.get("data");
    let data: QueryResult = serde_json::from_value(data.unwrap().clone()).unwrap();

    assert_eq!(data.features.len(), 1);

    test_utils::teardown().await.unwrap();

    // test api
}

#[tokio::test]
async fn test_api_profile() {
    let app = test_utils::setup_api(true).await.unwrap();

    let mut features = HashMap::new();
    features.insert(
        "feature1".to_string(),
        SpcFeatureDriftProfile {
            id: "feature1".to_string(),
            center: 0.0,
            one_ucl: 1.0,
            one_lcl: -1.0,
            two_ucl: 2.0,
            two_lcl: -2.0,
            three_ucl: 3.0,
            three_lcl: -3.0,
            timestamp: chrono::Utc::now().naive_utc(),
        },
    );

    let monitor_profile = SpcDriftProfile {
        features,
        config: SpcDriftConfig {
            sample_size: 100,
            sample: true,
            name: "test_app".to_string(),
            repository: "test".to_string(),
            version: "1.0.0".to_string(),
            targets: Vec::new(),
            feature_map: None,
            alert_config: SpcAlertConfig {
                rule: SpcAlertRule {
                    rule: "test".to_string(),
                    zones_to_monitor: Vec::new(),
                },
                dispatch_type: AlertDispatchType::Console,
                schedule: "0 0 * * * *".to_string(),
                features_to_monitor: Vec::new(),

                dispatch_kwargs: HashMap::new(),
            },
            drift_type: DriftType::SPC,
        },
        scouter_version: "1.0.0".to_string(),
    };

    let body = serde_json::to_string(&monitor_profile).unwrap();

    // insert data for new version
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scouter/profile")
                .header(http::header::CONTENT_TYPE, "application/json")
                .method("POST")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    test_utils::teardown().await.unwrap();
}

#[tokio::test]
async fn test_api_profile_update() {
    let app = test_utils::setup_api(true).await.unwrap();
    let pool = test_utils::setup_db(true).await.unwrap();

    // populate the database
    let populate_script = include_str!("scripts/populate.sql");
    sqlx::raw_sql(populate_script).execute(&pool).await.unwrap();

    // get current active status
    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_profile
        WHERE name = 'test_app'
        AND repository = 'mathworld'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let curr_status: bool = result[0].get("active");
    assert!(!curr_status);

    // put request
    let body = ProfileStatusRequest {
        name: "test_app".to_string(),
        repository: "mathworld".to_string(),
        version: "0.1.0".to_string(),
        active: true,
    };

    let body = serde_json::to_string(&body).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/scouter/profile/status")
                .header(http::header::CONTENT_TYPE, "application/json")
                .method("PUT")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // get current active status
    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_profile
        WHERE name = 'test_app'
        AND repository = 'mathworld'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let new_status: bool = result[0].get("active");

    assert!(new_status != curr_status);
    assert!(new_status);

    test_utils::teardown().await.unwrap();
}

#[tokio::test]
async fn test_api_get_drift_alert() {
    let app = test_utils::setup_api(true).await.unwrap();
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    // populate the database
    let populate_script = include_str!("scripts/populate.sql");
    sqlx::raw_sql(populate_script).execute(&pool).await.unwrap();
    let mut drift_executor = DriftExecutor::new(db_client.clone());

    drift_executor.poll_for_tasks().await.unwrap();
    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_profile
        WHERE name = 'test_app'
        AND repository = 'statworld'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(result.len(), 1);

    let cloned_app = app.clone();

    let response = cloned_app
        .oneshot(
            Request::builder()
                .uri("/scouter/alerts?name=test_app&repository=statworld&version=0.1.0")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // get data field from response
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();

    let data = body.get("data");
    let data: Vec<AlertResult> = serde_json::from_value(data.unwrap().clone()).unwrap();

    assert_eq!(data.len(), 2);

    test_utils::teardown().await.unwrap();
}

#[tokio::test]
async fn test_api_update_profile() {
    let app = test_utils::setup_api(true).await.unwrap();
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    // populate the database
    let populate_script = include_str!("scripts/populate.sql");
    sqlx::raw_sql(populate_script).execute(&pool).await.unwrap();
    let mut drift_executor = DriftExecutor::new(db_client.clone());

    drift_executor.poll_for_tasks().await.unwrap();
    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_profile
        WHERE name = 'test_app'
        AND repository = 'statworld'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(result.len(), 1);
    let updated_app = app.clone();
    let get_app = app.clone();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/scouter/profile?name=test_app&repository=statworld&version=0.1.0")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // convert to DriftProfile

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    let data = body.get("data").unwrap();
    let profile = serde_json::from_value::<SpcDriftProfile>(data.clone()).unwrap();
    assert!(profile.config.name == "test_app");

    let mut new_profile = profile.clone();
    new_profile.config.alert_config.rule = SpcAlertRule {
        rule: "8 8 10 10 8 8 1 1".to_string(),
        zones_to_monitor: Vec::new(),
    };

    let body = serde_json::to_string(&new_profile).unwrap();
    let response = updated_app
        .oneshot(
            Request::builder()
                .uri("/scouter/profile")
                .header(http::header::CONTENT_TYPE, "application/json")
                .method("PUT")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let response = get_app
        .oneshot(
            Request::builder()
                .uri("/scouter/profile?name=test_app&repository=statworld&version=0.1.0")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // convert to DriftProfile

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    let updated_profile = body.get("data").unwrap();
    let updated_profile =
        serde_json::from_value::<SpcDriftProfile>(updated_profile.clone()).unwrap();

    assert_eq!(
        updated_profile.config.alert_config.rule.rule,
        "8 8 10 10 8 8 1 1"
    );

    // ch

    test_utils::teardown().await.unwrap();
}
