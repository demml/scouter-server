use crate::api::schema::{DriftRecordRequest, ServiceDriftRequest};
use crate::sql::postgres::TimeInterval;
use crate::sql::schema::DriftRecord;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use scouter::utils::types::DriftProfile;
use serde_json::json;
use std::sync::Arc;
use tracing::error;

use crate::api::route::AppState;

pub async fn health_check() -> impl IntoResponse {
    const MESSAGE: &str = "Alive";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}

pub async fn get_drift(
    State(data): State<Arc<AppState>>,
    params: Query<ServiceDriftRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // validate time window

    let time_interval = TimeInterval::from_string(&params.time_window).to_minutes();

    let query_result = &data
        .db
        .get_binned_drift_records(
            &params.name,
            &params.repository,
            &params.version,
            &params.max_data_points,
            &time_interval,
        )
        .await;

    match query_result {
        Ok(result) => {
            let json_response = serde_json::json!({
                "status": "success",
                "data": result
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!("Failed to query drift records: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}

pub async fn insert_drift(
    State(data): State<Arc<AppState>>,
    Json(body): Json<DriftRecordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // set default if missing
    let record = DriftRecord {
        created_at: body
            .created_at
            .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
        name: body.name.clone(),
        repository: body.repository.clone(),
        feature: body.feature.clone(),
        value: body.value,
        version: body.version.clone(),
    };

    let query_result = &data.db.insert_drift_record(&record).await;

    match query_result {
        Ok(_) => {
            let json_response = json!({
                "status": "success",
                "message": "Record inserted successfully"
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!("Failed to insert drift record: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}

pub async fn insert_drift_profile(
    State(data): State<Arc<AppState>>,
    Json(body): Json<DriftProfile>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let query_result = &data.db.insert_drift_profile(&body).await;

    match query_result {
        Ok(_) => {
            let json_response = json!({
                "status": "success",
                "message": "Monitor profile inserted successfully"
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!("Failed to insert monitor profile: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}
