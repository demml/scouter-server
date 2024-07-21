use crate::sql::postgres::PostgresClient;
use crate::sql::schema::QueryResult;
use chrono::NaiveDateTime;
use scouter::core::alert::generate_alerts;
use scouter::core::monitor::Monitor;
use scouter::utils::types::DriftProfile;

use anyhow::{Context, Result};

use crate::alerts::dispatch::OpsGenieAlertDispatcher;
use ndarray::Array2;
use sqlx::{Postgres, Row};
use tracing::info;

#[derive(Debug)]
pub struct DriftExecutor {
    db_client: PostgresClient,
    ops_genie_alert_dispatcher: OpsGenieAlertDispatcher,
}

impl DriftExecutor {
    pub fn new(db_client: PostgresClient) -> Self {
        Self {
            db_client,
            ops_genie_alert_dispatcher: OpsGenieAlertDispatcher::default(),
        }
    }

    async fn get_drift_features(
        &self,
        profile: &DriftProfile,
        limit_time_stamp: &str,
    ) -> Result<QueryResult> {
        let records = self
            .db_client
            .get_drift_records(
                profile.config.name.as_str(),
                profile.config.repository.as_str(),
                profile.config.version.as_str(),
                limit_time_stamp,
            )
            .await?;
        Ok(records)
    }

    async fn compute_drift(
        &self,
        drift_profile: &DriftProfile,
        previous_run: &NaiveDateTime,
    ) -> Result<Array2<f64>> {
        let drift_features = self
            .get_drift_features(drift_profile, &previous_run.to_string())
            .await
            .with_context(|| "error retrieving raw feature data to compute drift")?;
        let feature_keys: Vec<String> = drift_features.features.keys().cloned().collect();
        let feature_values: Vec<f64> = drift_features
            .features
            .values()
            .cloned()
            .flat_map(|feature| feature.values.clone())
            .collect();
        let num_rows = drift_features.features.len();
        let num_cols = if num_rows > 0 {
            feature_values.len() / num_rows
        } else {
            0
        };
        let nd_feature_arr = Array2::from_shape_vec((num_rows, num_cols), feature_values)
            .with_context(|| "Shape error")?;
        let drift = Monitor::new().calculate_drift_from_sample(
            &feature_keys,
            &nd_feature_arr.view(),
            drift_profile,
        )?;
        Ok(drift)
    }

    pub async fn execute(&self) -> Result<()> {
        loop {
            let mut transaction: sqlx::Transaction<Postgres> = self.db_client.pool.begin().await?;
            // Get drift profile
            let profile = PostgresClient::get_drift_profile(&mut transaction)
                .await
                .with_context(|| "error retrieving drift profile(s) from db!")?;

            if let Some(profile) = profile {
                let drift_profile = serde_json::from_value::<DriftProfile>(profile.get("profile"))
                    .with_context(|| {
                        "error converting postgres jsonb profile to struct type DriftProfile"
                    })?;
                let previous_run: NaiveDateTime = profile.get("previous_run");
                let schedule: String = profile.get("schedule");
                // Compute drift
                let drift_array = self
                    .compute_drift(&drift_profile, &previous_run)
                    .await
                    .with_context(|| "error computing drift")?;

                // Get alerts
                let alerts = generate_alerts(
                    &drift_array.view(),
                    drift_profile.features.keys().cloned().collect(),
                    drift_profile.config.alert_rule,
                )
                .with_context(|| "error generating drift alerts")?;

                // Process alerts
                self.ops_genie_alert_dispatcher
                    .process_alerts(&alerts, &drift_profile.config.name)
                    .await?;

                // Update run dates for profile
                PostgresClient::update_drift_profile_run_dates(
                    &mut transaction,
                    &drift_profile.config.name,
                    &drift_profile.config.repository,
                    &drift_profile.config.version,
                    &schedule,
                )
                .await?;
            } else {
                info!("No more drift profiles to process, shutting down...");
                break;
            }
            transaction.commit().await?;
        }
        Ok(())
    }
}
