use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriftRecord {
    pub service_name: String,
    pub feature: String,
    pub value: f64,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct FeatureResult {
    pub created_at: Vec<chrono::NaiveDateTime>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub features: HashMap<String, FeatureResult>,
}