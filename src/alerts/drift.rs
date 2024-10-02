use crate::sql::postgres::PostgresClient;
use crate::sql::schema::QueryResult;
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use scouter::core::dispatch::dispatcher::dispatcher_logic::AlertDispatcher;
use scouter::core::drift::base::DriftType;
use scouter::core::drift::spc::alert::generate_alerts;
use scouter::core::drift::spc::monitor::SpcMonitor;
use scouter::core::drift::spc::types::SpcDriftProfile;
use tracing::error;
use tracing::info;

use crate::alerts::types::{Drifter, TaskAlerts};
use ndarray::Array2;
use sqlx::Row;
use std::collections::BTreeMap;

use super::spc::drift::SpcDrifter;

pub trait GetDrifter {
    fn get_drifter(
        &self,
        db_client: PostgresClient,
        name: String,
        repository: String,
        version: String,
    ) -> Drifter;
}

impl GetDrifter for SpcDriftProfile {
    fn get_drifter(
        &self,
        db_client: PostgresClient,
        name: String,
        repository: String,
        version: String,
    ) -> Drifter {
        Drifter::SpcDrifter(SpcDrifter::new(
            db_client,
            name,
            repository,
            version,
            self.clone(),
        ))
    }
}

pub struct DriftExecutor {
    db_client: PostgresClient,
}

impl DriftExecutor {
    pub fn new(db_client: PostgresClient) -> Self {
        Self { db_client }
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
    pub async fn process_task<T: GetDrifter>(
        &mut self,
        profile: T,
        previous_run: NaiveDateTime,
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<Option<Vec<BTreeMap<String, String>>>, anyhow::Error> {
        // match Drifter enum
        let drifter = profile.get_drifter(
            self.db_client.clone(),
            name.to_string(),
            repository.to_string(),
            version.to_string(),
        );

        drifter.check_for_alerts(previous_run).await
    }

    /// Execute single drift computation and alerting
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result of drift computation and alerting
    pub async fn poll_for_tasks(&mut self) -> Result<()> {
        let mut transaction = self.db_client.pool.begin().await?;

        // this will pull a drift profile from the db
        let task = PostgresClient::get_drift_profile_task(&mut transaction)
            .await
            .context("error retrieving drift profile(s) from db!")?;

        let Some(task) = task else {
            transaction.commit().await?;
            info!("No triggered schedules found in db. Sleeping for 10 seconds");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            return Ok(());
        };

        let profile = if task.profile_type == "spc" {
            serde_json::from_value::<SpcDriftProfile>(task.profile)
                .context("error converting postgres jsonb profile to struct type SpcDriftProfile")
        } else {
            Err(anyhow::anyhow!("unsupported profile type"))
        };

        if let Ok(profile) = profile {
            match self
                .process_task(
                    profile,
                    task.previous_run,
                    &task.name,
                    &task.repository,
                    &task.version,
                )
                .await
            {
                Ok(alerts) => {
                    info!("Drift task processed successfully");

                    if let Some(alerts) = alerts {
                        // insert each task into db
                        for alert in alerts {
                            if let Err(e) = self
                                .db_client
                                .insert_drift_alert(
                                    &task.name,
                                    &task.repository,
                                    &task.version,
                                    alert.get("feature").unwrap_or(&"NA".to_string()),
                                    &alert,
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
                "Error converting drift profile for {}/{}/{}",
                &task.repository, &task.name, &task.version
            );
        }

        if let Err(e) = PostgresClient::update_drift_profile_run_dates(
            &mut transaction,
            &task.name,
            &task.repository,
            &task.version,
            &task.schedule,
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
