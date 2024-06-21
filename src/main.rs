mod api;
mod kafka;
mod model;
mod sql;
use crate::api::route::AppState;
use crate::api::setup::{setup, setup_kafka_consumer};
use anyhow::Context;
use api::route::create_router;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // db for app state and kafka
    let db_client = setup()
        .await
        .with_context(|| "Failed to create Postgres client")?;

    // setup background task if kafka is enabled
    if std::env::var("KAFKA_BROKER").is_ok() {
        let brokers = std::env::var("KAFKA_BROKER").unwrap();
        let topics = vec![std::env::var("KAFKA_TOPIC").unwrap()];
        let group = std::env::var("KAFKA_GROUP").unwrap();

        setup_kafka_consumer(db_client.clone(), brokers, topics, group).await?;
    }

    // start server
    let app = create_router(Arc::new(AppState { db: db_client }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .with_context(|| "Failed to bind to port 8000")?;

    axum::serve(listener, app).await.unwrap();

    info!("🚀 Server started successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tokio;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn test_health_check() {
        let db_client = sql::postgres::PostgresClient::new(Some(
            "postgresql://postgres:admin@localhost:5432/monitor?".to_string(),
        ))
        .await
        .unwrap();

        let app = create_router(Arc::new(AppState {
            db: db_client.clone(),
        }));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthcheck")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        //assert response
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();

        let v: Value = serde_json::from_str(std::str::from_utf8(&body[..]).unwrap()).unwrap();
        let message: &str = v.get("message").unwrap().as_str().unwrap();

        assert_eq!(message, "Alive");
    }
}
