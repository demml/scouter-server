mod handler;
mod kafka;
mod model;

mod route;
mod sql;

use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber;

use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use route::create_router;
use tower_http::cors::CorsLayer;

pub struct AppState {
    db: Pool<Postgres>,
}

#[tokio::main]
async fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // get max connections from env or set to 10
    let max_connections = std::env::var("MAX_CONNECTIONS")
        .unwrap_or_else(|_| "10".to_string())
        .parse::<u32>()
        .expect("MAX_CONNECTIONS must be a number");

    // create a connection pool
    let pool = match PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            info!("✅ Successfully connected to database");
            pool
        }
        Err(err) => {
            error!("🔥 Failed to connect to database {:?}", err);
            std::process::exit(1);
        }
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::PUT, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let app = create_router(Arc::new(AppState { db: pool.clone() })).layer(cors);

    info!("🚀 Server started successfully");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
