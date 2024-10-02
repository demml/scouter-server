use crate::alerts::spc::drift::SpcDrifter;
use chrono::NaiveDateTime;
use scouter::core::drift::spc::types::SpcFeatureAlerts;
use std::collections::BTreeMap;

pub struct TaskAlerts {
    pub alerts: Option<SpcFeatureAlerts>,
}

pub enum Drifter {
    SpcDrifter(SpcDrifter),
}

impl Drifter {
    pub async fn check_for_alerts(
        &self,
        previous_run: NaiveDateTime,
    ) -> Result<Option<Vec<BTreeMap<String, String>>>, anyhow::Error> {
        match self {
            Drifter::SpcDrifter(drifter) => drifter.check_for_alerts(previous_run).await,
        }
    }
}
