use std::collections::BTreeMap;

use crate::sql::postgres::PostgresClient;
use crate::sql::schema::QueryResult;
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use ndarray::ArrayView2;
use scouter::core::dispatch::dispatcher::dispatcher_logic::AlertDispatcher;
use scouter::core::drift::spc::alert::generate_alerts;
use scouter::core::drift::spc::monitor::SpcMonitor;
use scouter::core::drift::spc::types::SpcDriftProfile;
use tracing::error;
use tracing::info;

use crate::alerts::types::TaskAlerts;
use ndarray::Array2;

// Defines the SpcDrifter struct
// This is used to process drift alerts for spc style profiles
pub struct SpcDrifter {
    db_client: PostgresClient,
    name: String,
    repository: String,
    version: String,
    profile: SpcDriftProfile,
}

impl SpcDrifter {
    pub fn new(
        db_client: PostgresClient,
        name: String,
        repository: String,
        version: String,
        profile: SpcDriftProfile,
    ) -> Self {
        Self {
            db_client,
            name,
            repository,
            version,
            profile,
        }
    }
    async fn get_drift_features(
        &self,
        limit_timestamp: &str,
        features_to_monitor: &[String],
    ) -> Result<QueryResult> {
        let records = self
            .db_client
            .get_drift_records(
                &self.name,
                &self.repository,
                &self.version,
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
        limit_timestamp: &NaiveDateTime,
    ) -> Result<(Array2<f64>, Vec<String>)> {
        let drift_features = self
            .get_drift_features(
                &limit_timestamp.to_string(),
                &self.profile.config.alert_config.features_to_monitor,
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

        let drift = SpcMonitor::new().calculate_drift_from_sample(
            &feature_keys,
            &nd_feature_arr.t().view(), // need to transpose because calculation is done at the row level across each feature
            &self.profile,
        )?;

        Ok((drift, feature_keys))
    }

    async fn generate_alerts<'a>(
        &self,
        array: &ArrayView2<'a, f64>,
        features: &[String],
        mut task_alerts: TaskAlerts,
    ) -> Result<TaskAlerts, anyhow::Error> {
        // Get alerts
        // keys are the feature names that match the order of the drift array columns
        let alert_rule = self.profile.config.alert_config.rule.clone();
        let alerts = generate_alerts(&array, &features, &alert_rule)
            .with_context(|| "error generating drift alerts")?;

        // Get dispatcher, will default to console if env vars are not found for 3rd party service
        // TODO: Add ability to pass hashmap of kwargs to dispatcher (from drift profile)
        // This would be for things like opsgenie team, feature priority, slack channel, etc.
        let alert_dispatcher = AlertDispatcher::new(&self.profile.config).map_err(|e| {
            error!(
                "Error creating alert dispatcher for {}/{}/{}: {}",
                self.repository, self.name, self.version, e
            );
            anyhow::anyhow!("Error creating alert dispatcher")
        })?;

        if alerts.has_alerts {
            alert_dispatcher
                .process_alerts(&alerts)
                .await
                .map_err(|e| {
                    error!(
                        "Error processing alerts for {}/{}/{}: {}",
                        self.repository, self.name, self.version, e
                    );
                    anyhow::anyhow!("Error processing alerts")
                })?;
            task_alerts.alerts = Some(alerts);
        } else {
            info!(
                "No alerts to process for {}/{}/{}",
                self.repository, self.name, self.version
            );
        }

        Ok(task_alerts)
    }

    /// organize alerts so that each alert is mapped to a single entry and feature
    /// Some features may produce multiple alerts
    ///
    /// # Arguments
    ///
    /// * `alerts` - TaskAlerts to organize
    ///
    /// # Returns
    ///
    fn organize_alerts(&self, alerts: TaskAlerts) -> Option<Vec<BTreeMap<String, String>>> {
        if let Some(mut alerts) = alerts.alerts {
            let mut tasks = Vec::new();
            alerts.features.iter_mut().for_each(|(_, feature)| {
                feature.alerts.iter().for_each(|alert| {
                    let alert_map = {
                        let mut alert_map = BTreeMap::new();
                        alert_map.insert("zone".to_string(), alert.zone.clone());
                        alert_map.insert("kind".to_string(), alert.kind.clone());
                        alert_map.insert("feature".to_string(), feature.feature.clone());
                        alert_map
                    };
                    tasks.push(alert_map);
                });
            });
            return Some(tasks);
        }
        None
    }

    /// Process a single drift computation task
    ///
    /// # Arguments
    /// * `previous_run` - Previous run timestamp
    pub async fn check_for_alerts(
        &self,
        previous_run: NaiveDateTime,
    ) -> Result<Option<Vec<BTreeMap<String, String>>>, anyhow::Error> {
        info!(
            "Processing drift task for profile: {}/{}/{}",
            self.repository, self.name, self.version
        );
        let alerts = TaskAlerts { alerts: None };

        // Compute drift
        let (drift_array, keys) = self
            .compute_drift(&previous_run)
            .await
            .with_context(|| "error computing drift")?;

        // if drift array is empty, return early
        if drift_array.is_empty() {
            info!("No features to process returning early");
            return Ok(None);
        }

        let alerts = self
            .generate_alerts(&drift_array.view(), &keys, alerts)
            .await
            .map_err(|e| {
                error!(
                    "Error generating alerts for {}/{}/{}: {}",
                    self.repository, self.name, self.version, e
                );
                anyhow::anyhow!("Error generating alerts")
            })?;

        Ok(self.organize_alerts(alerts))
    }
}
