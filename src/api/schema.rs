use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceDriftRequest {
    pub service_name: String,
    pub version: String,
    pub time_window: String,
    pub max_data_points: i32,
}
