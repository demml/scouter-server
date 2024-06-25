use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;

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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ProcessAlertRule {
    pub rule: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct PercentageAlertRule {
    pub rule: f64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct AlertRule {
    pub control: Option<ProcessAlertRule>,

    pub percentage: Option<PercentageAlertRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorConfig {
    pub sample_size: usize,

    pub sample: bool,

    pub name: String,

    pub repository: String,

    pub version: String,

    pub alert_rule: AlertRule,

    pub cron: String,
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
pub struct MonitorProfile {
    pub features: HashMap<String, FeatureMonitorProfile>,

    pub config: MonitorConfig,
}
