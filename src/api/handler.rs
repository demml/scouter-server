use crate::api::schema::{
    DriftAlertRequest, DriftRecordRequest, FeatureDriftDistributionRequest, ProfileRequest,
    ProfileStatusRequest, ServiceDriftRequest, UpdateAlertRequest,
};
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
use tracing::{error, info};

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
            params.feature.clone(),
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

pub async fn get_feature_distributions(
    State(data): State<Arc<AppState>>,
    params: Query<FeatureDriftDistributionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // validate time window

    let time_window = TimeInterval::from_string(&params.time_window).to_minutes();

    let query_result = &data
        .db
        .get_feature_distribution(
            &params.name,
            &params.repository,
            &params.version,
            &params.max_data_points,
            &time_window,
            &params.feature,
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
            error!("Failed to calculate feature distribution: {:?}", e);
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
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // validate profile is correct
    // this will be used to validate different versions of the drift profile in the future
    let body: Result<DriftProfile, serde_json::Error> = serde_json::from_value(body.clone());

    if body.is_err() {
        // future: - validate against older versions of the drift profile
        let json_response = json!({
            "status": "error",
            "message": "Invalid drift profile"
        });
        return Err((StatusCode::BAD_REQUEST, Json(json_response)));
    }

    let query_result = &data.db.insert_drift_profile(&body.unwrap()).await;

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

pub async fn update_drift_profile(
    State(data): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // validate profile is correct
    // this will be used to validate different versions of the drift profile in the future
    let body: Result<DriftProfile, serde_json::Error> = serde_json::from_value(body.clone());

    if body.is_err() {
        // future: - validate against older versions of the drift profile
        let json_response = json!({
            "status": "error",
            "message": "Invalid drift profile"
        });
        return Err((StatusCode::BAD_REQUEST, Json(json_response)));
    }

    let query_result = &data.db.update_drift_profile(&body.unwrap()).await;

    match query_result {
        Ok(_) => {
            let json_response = json!({
                "status": "success",
                "message": "Drift profile updated successfully"
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!("Failed to update drift profile: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}

pub async fn get_profile(
    State(data): State<Arc<AppState>>,
    params: Query<ProfileRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let profile = &data
        .db
        .get_drift_profile(&params.name, &params.repository, &params.version)
        .await;

    match profile {
        Ok(result) => {
            if result.is_some() {
                let json_response = json!({
                    "status": "success",
                    "profile": result
                });
                Ok(Json(json_response))
            } else {
                let json_response = json!({
                    "status": "error",
                    "detail": "Profile not found"
                });
                Ok(Json(json_response))
            }
        }
        Err(e) => {
            error!("Failed to query drift profile: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}

pub async fn update_drift_profile_status(
    State(data): State<Arc<AppState>>,
    Json(body): Json<ProfileStatusRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let query_result = &data
        .db
        .update_drift_profile_status(&body.name, &body.repository, &body.version, &body.active)
        .await;

    match query_result {
        Ok(_) => {
            let message = format!(
                "Monitor profile status updated to {} for {} {} {}",
                &body.active, &body.name, &body.repository, &body.version
            );
            let json_response = json!({
                "status": "success",
                "message": message
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!(
                "Failed to update drift profile status for {} {} {} : {:?}",
                &body.name, &body.repository, &body.version, e
            );
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}

pub async fn get_drift_alerts(
    State(data): State<Arc<AppState>>,
    params: Query<DriftAlertRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    info!("Querying drift alerts: {:?}", params);

    let query_result = &data
        .db
        .get_drift_alerts(
            &params.name,
            &params.repository,
            &params.version,
            params.limit_timestamp.as_deref(),
            params.active.unwrap_or(false),
            params.limit,
        )
        .await;

    match query_result {
        Ok(result) => {
            let json_response = json!({
                "status": "success",
                "data": result
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!("Failed to query drift alerts: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}

pub async fn update_drift_alerts(
    State(data): State<Arc<AppState>>,
    Json(body): Json<UpdateAlertRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let query_result = &data.db.update_drift_alert(body.id, body.status).await;

    match query_result {
        Ok(_) => {
            let json_response = json!({
                "status": "success",
                "message": "Drift alert updated successfully"
            });
            Ok(Json(json_response))
        }
        Err(e) => {
            error!("Failed to update drift alert: {:?}", e);
            let json_response = json!({
                "status": "error",
                "message": format!("{:?}", e)
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json_response)))
        }
    }
}
