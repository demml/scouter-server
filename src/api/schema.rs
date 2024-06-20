use chrono::NaiveDateTime;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceDriftRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub time_window: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureMonitorProfile {
    pub id: String,

    pub center: f64,

    pub one_ucl: f64,

    pub one_lcl: f64,

    pub two_ucl: f64,

    pub two_lcl: f64,

    pub three_ucl: f64,

    pub three_lcl: f64,

    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorConfig {
    pub sample_size: usize,

    pub sample: bool,

    pub name: String,

    pub repository: String,

    pub alert_rule: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorProfile {
    pub features: HashMap<String, FeatureMonitorProfile>,

    pub config: MonitorConfig,
}
