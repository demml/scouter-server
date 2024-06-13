mod api;
mod kafka;
mod model;
mod sql;

use anyhow::Context;
use kafka::consumer::ScouterConsumer;
use sql::postgres::PostgresClient;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Method,
};

use api::route::create_router;
use tower_http::cors::CorsLayer;

pub struct AppState {
    db: PostgresClient,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    // for app state
    let db_client = sql::postgres::PostgresClient::new()
        .await
        .with_context(|| "Failed to create Postgres client")?;

    // for background task (should have it's own pool)
    let loop_client = db_client.clone();

    let mut consumer = ScouterConsumer::new().with_context(|| "Failed to create Kafka consumer")?;

    // spawn the consumer as a background task
    tokio::spawn(async move {
        consumer.poll_loop(&loop_client).await;
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::PUT, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let app = create_router(Arc::new(AppState {
        db: db_client.clone(),
    }))
    .layer(cors);

    info!("🚀 Server started successfully");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .with_context(|| "Failed to bind to port 8000")?;

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
