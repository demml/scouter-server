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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResult {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub feature: String,
    pub alerts: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FeatureDistribution {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub percentile_10: f64,
    pub percentile_20: f64,
    pub percentile_30: f64,
    pub percentile_40: f64,
    pub percentile_50: f64,
    pub percentile_60: f64,
    pub percentile_70: f64,
    pub percentile_80: f64,
    pub percentile_90: f64,
    pub percentile_100: f64,
    pub val_10: f64,
    pub val_20: f64,
    pub val_30: f64,
    pub val_40: f64,
    pub val_50: f64,
    pub val_60: f64,
    pub val_70: f64,
    pub val_80: f64,
    pub val_90: f64,
    pub val_100: f64,
}
