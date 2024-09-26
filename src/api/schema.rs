use chrono::NaiveDateTime;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceDriftRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub time_window: String,
    pub feature: Option<String>,
    pub max_data_points: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeatureDriftDistributionRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub time_window: String,
    pub feature: String,
    pub max_data_points: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriftRecordRequest {
    pub created_at: Option<NaiveDateTime>,
    pub name: String,
    pub repository: String,
    pub feature: String,
    pub value: f64,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileStatusRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriftAlertRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub limit_timestamp: Option<String>,
    pub active: Option<bool>,
    pub limit: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateAlertRequest {
    pub id: i32,
    pub status: String,
}
