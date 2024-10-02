use crate::alerts::spc::drift::SpcDrifter;
use crate::sql::postgres::PostgresClient;
use chrono::NaiveDateTime;
use scouter::core::drift::base::DriftType;
use scouter::core::drift::spc::types::{SpcDriftProfile, SpcFeatureAlerts};
use std::collections::BTreeMap;
pub struct TaskAlerts {
    pub alerts: SpcFeatureAlerts,
}

impl TaskAlerts {
    pub fn new() -> Self {
        Self {
            alerts: SpcFeatureAlerts::new(false),
        }
    }
}

pub enum DriftProfile {
    SpcDriftProfile(SpcDriftProfile),
}

impl DriftProfile {
    pub fn from_type(drift_type: DriftType, profile: String) -> Result<Self, anyhow::Error> {
        match drift_type {
            DriftType::SPC => {
                let profile = serde_json::from_str(&profile)?;
                Ok(DriftProfile::SpcDriftProfile(profile))
            }
            DriftType::PSI => todo!(),
            DriftType::NONE => todo!(),
        }
    }
    pub fn get_drifter(&self, name: String, repository: String, version: String) -> Drifter {
        match self {
            DriftProfile::SpcDriftProfile(profile) => {
                Drifter::SpcDrifter(SpcDrifter::new(name, repository, version, profile.clone()))
            }
        }
    }
}

pub enum Drifter {
    SpcDrifter(SpcDrifter),
}

impl Drifter {
    pub async fn check_for_alerts(
        &self,
        db_client: &PostgresClient,
        previous_run: NaiveDateTime,
    ) -> Result<Option<Vec<BTreeMap<String, String>>>, anyhow::Error> {
        match self {
            Drifter::SpcDrifter(drifter) => drifter.check_for_alerts(db_client, previous_run).await,
        }
    }
}
