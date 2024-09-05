use crate::sql::postgres::PostgresClient;
use crate::sql::schema::QueryResult;
use chrono::NaiveDateTime;
use scouter::core::alert::generate_alerts;
use scouter::core::monitor::Monitor;
use scouter::utils::types::{AlertDispatchType, DriftProfile};

use anyhow::{Context, Result};

use crate::alerts::dispatch::{
    AlertDispatcher, ConsoleAlertDispatcher, HttpAlertDispatcher, OpsGenieAlerter, SlackAlerter,
};
use ndarray::Array2;
use sqlx::{Postgres, Row};

pub struct DriftExecutor {
    db_client: PostgresClient,
    alert_dispatcher: AlertDispatcher,
}

impl DriftExecutor {
    pub fn new(db_client: PostgresClient) -> Self {
        Self {
            db_client,
            alert_dispatcher: AlertDispatcher::Console(ConsoleAlertDispatcher),
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

    /// Compute drift for a given drift profile
    ///
    /// # Arguments
    ///
    /// * `drift_profile` - Drift profile to compute drift for
    /// * `limit_timestamp` - Limit timestamp for drift computation (this is the previous_run timestamp)
    ///     
    /// # Returns
    ///
    /// * `Result<Array2<f64>>` - Drift array
    pub async fn compute_drift(
        &self,
        drift_profile: &DriftProfile,
        limit_timestamp: &NaiveDateTime,
    ) -> Result<(Array2<f64>, Vec<String>)> {
        let drift_features = self
            .get_drift_features(drift_profile, &limit_timestamp.to_string())
            .await
            .with_context(|| "error retrieving raw feature data to compute drift")?;

        let feature_keys: Vec<String> = drift_features.features.keys().cloned().collect();
        let feature_values = drift_features
            .features
            .values()
            .cloned()
            .flat_map(|feature| feature.values.clone())
            .collect::<Vec<_>>();

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
            &nd_feature_arr.t().view(), // need to transpose because calculation is done at the row level across each feature
            drift_profile,
        )?;

        Ok((drift, feature_keys))
    }
    fn map_dispatcher(dispatch_type: &AlertDispatchType) -> AlertDispatcher {
        match dispatch_type {
            AlertDispatchType::Console => AlertDispatcher::Console(ConsoleAlertDispatcher),
            AlertDispatchType::OpsGenie => {
                AlertDispatcher::OpsGenie(HttpAlertDispatcher::new(OpsGenieAlerter::default()))
            }
            AlertDispatchType::Slack => {
                AlertDispatcher::Slack(HttpAlertDispatcher::new(SlackAlerter::default()))
            }
            AlertDispatchType::Email => panic!("Unsupported dispatcher type: Email"),
        }
    }

    pub async fn execute(&mut self) -> Result<()> {
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

            // switch back to previous run
            let previous_run: NaiveDateTime = profile.get("previous_run");
            let schedule: String = profile.get("schedule");

            // Compute drift
            let (drift_array, keys) = self
                .compute_drift(&drift_profile, &previous_run)
                .await
                .with_context(|| "error computing drift")?;

            // Get alerts
            // keys are the feature names that match the order of the drift array columns
            let alerts = generate_alerts(
                &drift_array.view(),
                keys,
                drift_profile.config.alert_config.alert_rule,
            )
            .with_context(|| "error generating drift alerts")?;

            // Check if non-default "Console" dispatcher type specified
            self.alert_dispatcher =
                Self::map_dispatcher(&drift_profile.config.alert_config.alert_dispatch_type);

            // Process alerts
            self.alert_dispatcher
                .process_alerts(
                    &alerts,
                    &drift_profile.config.repository,
                    &drift_profile.config.name,
                )
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
        }

        transaction.commit().await?;

        Ok(())
    }
}
