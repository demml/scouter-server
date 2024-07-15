mod alerts;
mod api;
mod kafka;
mod sql;

use crate::alerts::dispatch;
use crate::alerts::dispatch::OpsGenieAlertDispatcher;
use crate::alerts::drift::DriftExecutor;
use crate::api::route::AppState;
use crate::api::setup::{create_db_pool, setup_logging};
use crate::kafka::consumer::start_kafka_background_poll;
use crate::sql::postgres::PostgresClient;
use anyhow::Context;
use api::route::create_router;
use futures::executor::block_on;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use sqlx::{Pool, Postgres};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, error};

const NUM_WORKERS: usize = 5;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // setup logging
    setup_logging()
        .await
        .with_context(|| "Failed to setup logging")?;

    // db for app state and kafka
    let pool = create_db_pool(None)
        .await
        .with_context(|| "Failed to create Postgres client")?;

    // // run migrations
    // sqlx::migrate!()
    //     .run(&pool)
    //     .await
    //     .with_context(|| "Failed to run migrations")?;

    // setup background task if kafka is enabled
    if std::env::var("KAFKA_BROKER").is_ok() {
        let brokers = std::env::var("KAFKA_BROKER").unwrap();
        let topics = vec![std::env::var("KAFKA_TOPIC").unwrap_or("scouter_monitoring".to_string())];
        let group_id = std::env::var("KAFKA_GROUP").unwrap_or("scouter".to_string());
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
                let kafka_db_client = PostgresClient::new(pool.clone())
                    .with_context(|| "Failed to create Postgres client")
                    .unwrap();
                let message_handler = kafka::consumer::MessageHandler::Postgres(kafka_db_client);
                tokio::spawn(start_kafka_background_poll(
                    message_handler,
                    group_id.clone(),
                    brokers.clone(),
                    topics.clone(),
                    username.clone(),
                    password.clone(),
                    security_protocol.clone(),
                    sasl_mechanism.clone(),
                ))
            })
            .collect::<FuturesUnordered<_>>()
            .for_each(|_| async {});
    }

    // run drift background task
    let alert_db_client =
        PostgresClient::new(pool.clone()).with_context(|| "Failed to create Postgres client")?;
    tokio::task::spawn(async move {
        let drift_executor = DriftExecutor::new(alert_db_client);
        let mut interval = tokio::time::interval(Duration::from_secs(4));
        loop {
            interval.tick().await;
            if let Err(e) = drift_executor.execute().await {
                error!("Drift Executor Error: {e}")
            }
        }
    });

    // start server
    let server_db_client =
        PostgresClient::new(pool.clone()).with_context(|| "Failed to create Postgres client")?;

    let app = create_router(Arc::new(AppState {
        db: server_db_client,
    }));

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
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn test_health_check() {
        let pool = create_db_pool(Some(
            "postgresql://postgres:admin@localhost:5432/monitor?".to_string(),
        ))
        .await
        .with_context(|| "Failed to create Postgres client")
        .unwrap();

        let db_client = sql::postgres::PostgresClient::new(pool).unwrap();

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
