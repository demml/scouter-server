use crate::api::handler::{
    get_drift, get_drift_alerts, health_check, insert_drift, insert_drift_profile,
    update_drift_profile_status,
};
use crate::api::metrics::track_metrics;
use crate::sql::postgres::PostgresClient;
use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Method,
};
use axum::middleware;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub struct AppState {
    pub db: PostgresClient,
}

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::PUT, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    Router::new()
        .route("/healthcheck", get(health_check))
        .route("/drift", get(get_drift).post(insert_drift))
        .route("/profile", post(insert_drift_profile))
        .route("/profile/status", put(update_drift_profile_status))
        .route("/alerts", get(get_drift_alerts))
        .route_layer(middleware::from_fn(track_metrics))
        .with_state(app_state)
        .layer(cors)
}
