use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceDriftRequest {
    pub service_name: String,
    pub version: String,
    pub time_window: String,
    pub max_data_points: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriftRecord {
    pub service_name: String,
    pub feature: String,
    pub value: f64,
    pub version: String,
}
