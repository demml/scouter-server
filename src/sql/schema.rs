use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriftRecord {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub feature: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureResult {
    pub created_at: Vec<chrono::NaiveDateTime>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub features: BTreeMap<String, FeatureResult>,
}
