use scouter_server::api;
use scouter_server::api::route::{create_router, AppState};
use scouter_server::sql::schema::{DriftRecord, QueryResult};
mod common;
use axum::{
    body::Body,
    http::{self, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tokio;
use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

#[tokio::test]
async fn test_api_read() {
    let db_client = common::setup_for_api().await.unwrap();

    // test setting up kafka consumer
    api::setup::setup_kafka_consumer(db_client.clone());

    let app = create_router(Arc::new(AppState {
        db: db_client.clone(),
    }));

    let response = app.oneshot(
        Request::builder()
            .uri("/drift?service_name=test_app&version=1.0.0&time_window=5minute&max_data_points=1000")
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
    assert_eq!(data.features.len(), 3);

    let app = create_router(Arc::new(AppState {
        db: db_client.clone(),
    }));

    let record = DriftRecord {
        service_name: "test_app".to_string(),
        feature: "feature1".to_string(),
        value: 2.5,
        version: "2.0.0".to_string(),
    };

    let body = serde_json::to_string(&record).unwrap();

    // insert data for new version
    let response = app
        .oneshot(
            Request::builder()
                .uri("/drift")
                .header(http::header::CONTENT_TYPE, "application/json")
                .method("POST")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let app = create_router(Arc::new(AppState {
        db: db_client.clone(),
    }));

    let response = app.oneshot(
        Request::builder()
            .uri("/drift?service_name=test_app&version=2.0.0&time_window=5minute&max_data_points=1000")
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
    // teardown
    common::teardown(&db_client).await.unwrap();

    // test api
}
