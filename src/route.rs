use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{handler::health_check, AppState};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthcheck", get(health_check))
        .with_state(app_state)
}
