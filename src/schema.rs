use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriftRecord {
    pub service_name: String,
    pub feature: String,
    pub value: f64,
    pub version: String,
}

pub struct FeatureResult {
    pub created_at: Vec<String>,
    pub values: Vec<f64>,
}

pub struct QueryResult {
    pub features: HashMap<String, FeatureResult>,
}
