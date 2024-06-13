use std::sync::Arc;

use axum::{routing::get, Router};

use crate::{
    api::handler::{get_drift, health_check},
    AppState,
};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthcheck", get(health_check))
        .route("/drift", get(get_drift))
        .with_state(app_state)
}
