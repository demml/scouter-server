use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PutDriftRequest {
    pub service_name: String,
    pub feature: String,
    pub value: f64,
}
