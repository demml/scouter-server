use crate::sql::postgres::PostgresClient;
use crate::sql::schema::QueryResult;
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use scouter::core::alert::generate_alerts;
use scouter::core::monitor::Monitor;
use scouter::utils::types::DriftProfile;
use tracing::error;
use tracing::info;

use crate::alerts::dispatch::AlertDispatcher;
use ndarray::Array2;
use sqlx::{Postgres, Row};

pub struct DriftExecutor {
    db_client: PostgresClient,
}

impl DriftExecutor {
    pub fn new(db_client: PostgresClient) -> Self {
        Self { db_client }
    }

    async fn get_drift_features(
        &self,
        name: &str,
        repository: &str,
        version: &str,
        limit_time_stamp: &str,
    ) -> Result<QueryResult> {
        let records = self
            .db_client
            .get_drift_records(name, repository, version, limit_time_stamp)
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
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<(Array2<f64>, Array2<f64>, Vec<String>)> {
        let drift_features = self
            .get_drift_features(name, repository, version, &limit_timestamp.to_string())
            .await
            .with_context(|| "error retrieving raw feature data to compute drift")?;

        let feature_keys: Vec<String> = drift_features.features.keys().cloned().collect();
        let feature_values = drift_features
            .features
            .values()
            .cloned()
            .flat_map(|feature| feature.values.clone())
            .collect::<Vec<_>>();

        // assert all drift features have the same number of values

        let all_same_len = drift_features.features.iter().all(|(_, feature)| {
            feature.values.len()
                == drift_features
                    .features
                    .values()
                    .next()
                    .unwrap()
                    .values
                    .len()
        });

        if !all_same_len {
            return Err(anyhow::anyhow!("Feature values have different lengths"));
        }

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

        Ok((drift, nd_feature_arr, feature_keys))
    }

    /// Process a single drift computation task
    ///
    /// # Arguments
    ///
    /// * `drift_profile` - Drift profile to compute drift for
    /// * `previous_run` - Previous run timestamp
    /// * `schedule` - Schedule for drift computation
    /// * `transaction` - Postgres transaction
    ///
    /// # Returns
    ///
    pub async fn process_task<'a>(
        &mut self,
        drift_profile: DriftProfile,
        previous_run: NaiveDateTime,
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<(), anyhow::Error> {
        info!(
            "Processing drift task for profile: {}/{}/{}",
            repository, name, version
        );

        // Compute drift
        let (drift_array, sample_array, keys) = self
            .compute_drift(&drift_profile, &previous_run, name, repository, version)
            .await
            .with_context(|| "error computing drift")?;

        // if drift array is empty, return early
        if drift_array.is_empty() {
            info!("No features to process returning early");
            return Ok(());
        }

        // Get alerts
        // keys are the feature names that match the order of the drift array columns
        let alert_rule = drift_profile.config.alert_config.alert_rule.clone();
        let alerts = generate_alerts(
            &drift_array.view(),
            &sample_array.view(),
            &keys,
            &alert_rule,
        )
        .with_context(|| "error generating drift alerts")?;

        // Get dispatcher, will default to console if env vars are not found for 3rd party service
        // TODO: Add ability to pass hashmap of kwargs to dispatcher (from drift profile)
        // This would be for things like opsgenie team, feature priority, slack channel, etc.
        let alert_dispatcher = AlertDispatcher::new(&drift_profile.config);

        alert_dispatcher
            .process_alerts(&alerts)
            .await
            .with_context(|| "error processing alerts")?;

        Ok(())
    }

    /// Execute single drift computation and alerting
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result of drift computation and alerting
    pub async fn poll_for_tasks(&mut self) -> Result<()> {
        let mut sleep: bool = false;
        let mut transaction: sqlx::Transaction<Postgres> = self.db_client.pool.begin().await?;

        // Get drift profile
        let task = PostgresClient::get_drift_profile(&mut transaction)
            .await
            .with_context(|| "error retrieving drift profile(s) from db!")?;

        if let Some(task) = task {
            // parse task args
            let name: String = task.get("name");
            let repository: String = task.get("repository");
            let version: String = task.get("version");
            let schedule: String = task.get("schedule");

            let drift_profile = serde_json::from_value::<DriftProfile>(task.get("profile"))
                .with_context(|| {
                    "error converting postgres jsonb profile to struct type DriftProfile"
                })?;

            // Process task
            let result = self
                .process_task(
                    drift_profile,
                    task.get("previous_run"),
                    &name,
                    &repository,
                    &version,
                )
                .await;

            match result {
                Ok(_) => {
                    info!("Drift task processed successfully");
                }
                Err(e) => {
                    error!("Error processing drift task: {:?}", e);
                }
            }

            // Update run dates for profile
            PostgresClient::update_drift_profile_run_dates(
                &mut transaction,
                &name,
                &repository,
                &version,
                &schedule,
            )
            .await?;
        } else {
            sleep = true;
        }

        // close transaction
        transaction.commit().await?;

        // sleep if no records found (no need to keep polling db)
        if sleep {
            // Sleep for a minute
            info!("No triggered schedules found in db. Sleeping for 10 seconds");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        Ok(())
    }
}
