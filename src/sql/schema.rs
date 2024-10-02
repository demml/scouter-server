use crate::api::schema::DriftRecordRequest;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{Error, FromRow, Row};
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

impl DriftRecord {
    //static method
    pub fn from_request(request: DriftRecordRequest) -> Self {
        DriftRecord {
            created_at: request
                .created_at
                .unwrap_or_else(|| chrono::Utc::now().naive_utc()),
            name: request.name,
            repository: request.repository,
            version: request.version,
            feature: request.feature,
            value: request.value,
        }
    }
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
