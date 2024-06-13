use kafka::producer::Record;
use scouter_server::api::route::{create_router, AppState};
use scouter_server::sql::schema::DriftRecord;
mod common;
use axum::{
    body::{self, Body},
    extract::connect_info::MockConnectInfo,
    http::{self, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tokio;
use tower::{Service, ServiceExt}; // for `call`, `oneshot`, and `ready`

#[tokio::test]
async fn test_api_read() {
    let (db_client, mut scouter_consumer, mut producer) = common::setup().await.unwrap();

    // feature name vec
    let feature_names = vec!["feature1", "feature2", "feature3"];

    // for each features, generate 1000 records
    feature_names.iter().for_each(|feature_name| {
        for i in 0..1000 {
            let record = DriftRecord {
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

    let app = create_router(Arc::new(AppState {
        db: db_client.clone(),
    }));

    // test api
}
