use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Error, FromRow, Row};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpcFeatureResult {
    pub feature: String,
    pub created_at: Vec<chrono::NaiveDateTime>,
    pub values: Vec<f64>,
}

impl<'r> FromRow<'r, PgRow> for SpcFeatureResult {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        Ok(SpcFeatureResult {
            feature: row.try_get("feature")?,
            created_at: row.try_get("created_at")?,
            values: row.try_get("values")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResult {
    pub created_at: NaiveDateTime,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub feature: String,
    pub alert: BTreeMap<String, String>,
    pub id: i32,
    pub status: String,
}

impl<'r> FromRow<'r, PgRow> for AlertResult {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        let alert_value: serde_json::Value = row.try_get("alert")?;
        let alert: BTreeMap<String, String> =
            serde_json::from_value(alert_value).unwrap_or_default();

        Ok(AlertResult {
            created_at: row.try_get("created_at")?,
            name: row.try_get("name")?,
            repository: row.try_get("repository")?,
            version: row.try_get("version")?,
            alert,
            feature: row.try_get("feature")?,
            id: row.try_get("id")?,
            status: row.try_get("status")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub profile: String,
    pub drift_type: String,
    pub previous_run: NaiveDateTime,
    pub schedule: String,
}

impl<'r> FromRow<'r, PgRow> for TaskRequest {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        let profile: serde_json::Value = row.try_get("profile")?;

        Ok(TaskRequest {
            name: row.try_get("name")?,
            repository: row.try_get("repository")?,
            version: row.try_get("version")?,
            profile: profile.to_string(),
            drift_type: row.try_get("drift_type")?,
            previous_run: row.try_get("previous_run")?,
            schedule: row.try_get("schedule")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityResult {
    pub route_name: String,
    pub created_at: Vec<chrono::NaiveDateTime>,
    pub p5: Vec<f64>,
    pub p25: Vec<f64>,
    pub p50: Vec<f64>,
    pub p95: Vec<f64>,
    pub p99: Vec<f64>,
    pub total_request_count: Vec<i64>,
    pub total_error_count: Vec<i64>,
    pub error_latency: Vec<f64>,
    pub status_counts: Vec<HashMap<String, i64>>,
}

impl<'r> FromRow<'r, PgRow> for ObservabilityResult {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        // decode status counts to vec of jsonb
        let status_counts: Vec<serde_json::Value> = row.try_get("status_counts")?;

        // convert vec of jsonb to vec of hashmaps
        let status_counts: Vec<HashMap<String, i64>> = status_counts
            .into_iter()
            .map(|value| serde_json::from_value(value).unwrap_or_default())
            .collect();

        Ok(ObservabilityResult {
            route_name: row.try_get("route_name")?,
            created_at: row.try_get("created_at")?,
            p5: row.try_get("p5")?,
            p25: row.try_get("p25")?,
            p50: row.try_get("p50")?,
            p95: row.try_get("p95")?,
            p99: row.try_get("p99")?,
            total_request_count: row.try_get("total_request_count")?,
            total_error_count: row.try_get("total_error_count")?,
            error_latency: row.try_get("error_latency")?,
            status_counts,
        })
    }
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

impl<'r> FromRow<'r, PgRow> for FeatureDistribution {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        Ok(FeatureDistribution {
            name: row.try_get("name")?,
            repository: row.try_get("repository")?,
            version: row.try_get("version")?,
            percentile_10: row.try_get("percentile_10")?,
            percentile_20: row.try_get("percentile_20")?,
            percentile_30: row.try_get("percentile_30")?,
            percentile_40: row.try_get("percentile_40")?,
            percentile_50: row.try_get("percentile_50")?,
            percentile_60: row.try_get("percentile_60")?,
            percentile_70: row.try_get("percentile_70")?,
            percentile_80: row.try_get("percentile_80")?,
            percentile_90: row.try_get("percentile_90")?,
            percentile_100: row.try_get("percentile_100")?,
            val_10: row.try_get("val_10")?,
            val_20: row.try_get("val_20")?,
            val_30: row.try_get("val_30")?,
            val_40: row.try_get("val_40")?,
            val_50: row.try_get("val_50")?,
            val_60: row.try_get("val_60")?,
            val_70: row.try_get("val_70")?,
            val_80: row.try_get("val_80")?,
            val_90: row.try_get("val_90")?,
            val_100: row.try_get("val_100")?,
        })
    }
}
