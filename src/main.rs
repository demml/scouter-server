mod api;
mod kafka;
mod model;
mod sql;
use crate::api::route::AppState;
use anyhow::Context;
use api::route::create_router;
use kafka::consumer::ScouterConsumer;
use sql::postgres::PostgresClient;
use std::io;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber;
use tracing_subscriber::fmt::time::UtcTime;

const DEFAULT_TIME_PATTERN: &str =
    "[year]-[month]-[day]T[hour repr:24]:[minute]:[second]::[subsecond digits:4]";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // install global collector configured based on RUST_LOG env var.
    let time_format = time::format_description::parse(DEFAULT_TIME_PATTERN).unwrap();

    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .flatten_event(true)
        .with_thread_ids(true)
        .with_timer(UtcTime::new(time_format))
        .with_writer(io::stdout)
        .init();

    // for app state
    let db_client = PostgresClient::new(None)
        .await
        .with_context(|| "Failed to create Postgres client")?;

    // for background task (should have it's own pool)
    let loop_client = db_client.clone();

    let mut consumer = ScouterConsumer::new().with_context(|| "Failed to create Kafka consumer")?;

    // spawn the consumer as a background task
    tokio::spawn(async move {
        consumer.poll_loop(&loop_client).await;
    });

    let app = create_router(Arc::new(AppState {
        db: db_client.clone(),
    }));

    info!("🚀 Server started successfully");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .with_context(|| "Failed to bind to port 8000")?;

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{self, Body},
        extract::connect_info::MockConnectInfo,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tokio;
    use tower::{Service, ServiceExt}; // for `call`, `oneshot`, and `ready`

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
