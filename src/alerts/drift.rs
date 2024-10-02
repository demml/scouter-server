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
use crate::alerts::types::TaskAlerts;
use ndarray::Array2;
use sqlx::Row;
use std::collections::BTreeMap;

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
        limit_timestamp: &str,
        features_to_monitor: &[String],
    ) -> Result<QueryResult> {
        let records = self
            .db_client
            .get_drift_records(
                name,
                repository,
                version,
                limit_timestamp,
                features_to_monitor,
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
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<(Array2<f64>, Vec<String>)> {
        let drift_features = self
            .get_drift_features(
                name,
                repository,
                version,
                &limit_timestamp.to_string(),
                &drift_profile.config.alert_config.features_to_monitor,
            )
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

        Ok((drift, feature_keys))
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
    pub async fn process_task(
        &mut self,
        drift_profile: DriftProfile,
        previous_run: NaiveDateTime,
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<TaskAlerts, anyhow::Error> {
        info!(
            "Processing drift task for profile: {}/{}/{}",
            repository, name, version
        );
        let mut task_alerts = TaskAlerts { alerts: None };

        // Compute drift
        let (drift_array, keys) = self
            .compute_drift(&drift_profile, &previous_run, name, repository, version)
            .await
            .with_context(|| "error computing drift")?;

        // if drift array is empty, return early
        if drift_array.is_empty() {
            info!("No features to process returning early");
            return Ok(task_alerts);
        }

        // Get alerts
        // keys are the feature names that match the order of the drift array columns
        let alert_rule = drift_profile.config.alert_config.alert_rule.clone();
        let alerts = generate_alerts(&drift_array.view(), &keys, &alert_rule)
            .with_context(|| "error generating drift alerts")?;

        // Get dispatcher, will default to console if env vars are not found for 3rd party service
        // TODO: Add ability to pass hashmap of kwargs to dispatcher (from drift profile)
        // This would be for things like opsgenie team, feature priority, slack channel, etc.
        let alert_dispatcher = AlertDispatcher::new(&drift_profile.config);

        if alerts.has_alerts {
            alert_dispatcher
                .process_alerts(&alerts)
                .await
                .with_context(|| "error processing alerts")?;
            task_alerts.alerts = Some(alerts);
        } else {
            info!(
                "No alerts to process for {}/{}/{}",
                repository, name, version
            );
        }

        Ok(task_alerts)
    }

    /// Execute single drift computation and alerting
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result of drift computation and alerting
    pub async fn poll_for_tasks(&mut self) -> Result<()> {
        let mut transaction = self.db_client.pool.begin().await?;

        let task = PostgresClient::get_drift_profile_task(&mut transaction)
            .await
            .context("error retrieving drift profile(s) from db!")?;

        let Some(task) = task else {
            transaction.commit().await?;
            info!("No triggered schedules found in db. Sleeping for 10 seconds");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            return Ok(());
        };

        let name = task.get::<String, _>("name");
        let repository = task.get::<String, _>("repository");
        let version = task.get::<String, _>("version");
        let schedule = task.get::<String, _>("schedule");

        let drift_profile = serde_json::from_value::<DriftProfile>(task.get("profile"))
            .context("error converting postgres jsonb profile to struct type DriftProfile");

        if let Ok(drift_profile) = drift_profile {
            match self
                .process_task(
                    drift_profile,
                    task.get("previous_run"),
                    &name,
                    &repository,
                    &version,
                )
                .await
            {
                Ok(task_alert) => {
                    info!("Drift task processed successfully");

                    if let Some(mut task_alert) = task_alert.alerts {
                        let mut tasks = Vec::new();
                        task_alert.features.iter_mut().for_each(|(_, feature)| {
                            feature.alerts.iter().for_each(|alert| {
                                let alert_map = {
                                    let mut alert_map = BTreeMap::new();
                                    alert_map.insert("zone".to_string(), alert.zone.clone());
                                    alert_map.insert("kind".to_string(), alert.kind.clone());
                                    alert_map
                                        .insert("feature".to_string(), feature.feature.clone());
                                    alert_map
                                };
                                tasks.push(alert_map);
                            });
                        });

                        // insert each task into db
                        for task in tasks {
                            if let Err(e) = self
                                .db_client
                                .insert_drift_alert(
                                    &name,
                                    &repository,
                                    &version,
                                    task.get("feature").unwrap_or(&"NA".to_string()),
                                    &task,
                                )
                                .await
                            {
                                error!("Error inserting drift alerts: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error processing drift task: {:?}", e);
                }
            }
        } else {
            error!(
                "Error converting drift profile for {}/{}/{}. Error: {:?}",
                &repository, &name, &version, drift_profile
            );
        }

        if let Err(e) = PostgresClient::update_drift_profile_run_dates(
            &mut transaction,
            &name,
            &repository,
            &version,
            &schedule,
        )
        .await
        {
            error!("Error updating drift profile run dates: {:?}", e);
        } else {
            info!("Drift profile run dates updated successfully");
        }

        transaction.commit().await?;

        Ok(())
    }
}
