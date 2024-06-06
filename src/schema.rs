use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DriftRecord {
    pub service_name: String,
    pub feature: String,
    pub value: f64,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Alert {
    pub alert_type: String,
    pub zone: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AlertRecord {
    pub service_name: String,
    pub alert: Alert,
}
