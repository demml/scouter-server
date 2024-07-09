mod api;
mod kafka;
mod model;
mod sql;
use crate::api::route::AppState;
use crate::api::setup::setup;
use crate::kafka::consumer::{setup_kafka_consumer, MessageHandler};
use anyhow::Context;
use api::route::create_router;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::sync::Arc;
use tracing::info;

const NUM_WORKERS: usize = 5;

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
        let username: Option<String> = std::env::var("KAFKA_USERNAME").ok();
        let password: Option<String> = std::env::var("KAFKA_PASSWORD").ok();
        let security_protocol: Option<String> = Some(
            std::env::var("KAFKA_SECURITY_PROTOCOL")
                .ok()
                .unwrap_or_else(|| "SASL_SSL".to_string()),
        );
        let sasl_mechanism: Option<String> = Some(
            std::env::var("KAFKA_SASL_MECHANISM")
                .ok()
                .unwrap_or_else(|| "PLAIN".to_string()),
        );

        let _background = (0..NUM_WORKERS)
            .map(|_| {
                let message_handler = MessageHandler::Postgres(db_client.clone());
                tokio::spawn(setup_kafka_consumer(
                    group.clone(),
                    brokers.clone(),
                    topics.clone(),
                    message_handler,
                    username.clone(),
                    password.clone(),
                    security_protocol.clone(),
                    sasl_mechanism.clone(),
                ))
            })
            .collect::<FuturesUnordered<_>>()
            .for_each(|_| async {});
    }

    let a = db_client.pool.clone();

    // start server
    let app = create_router(Arc::new(AppState { db: db_client }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .with_context(|| "Failed to bind to port 8000")?;

    info!("🚀 Server started successfully");
    axum::serve(listener, app).await.unwrap();

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
