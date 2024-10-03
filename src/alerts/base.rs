use crate::alerts::spc::drift::SpcDrifter;
use crate::alerts::types::Drifter;
use crate::sql::postgres::PostgresClient;
use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use scouter::core::drift::base::DriftType;
use scouter::core::drift::spc::types::SpcDriftProfile;
use std::collections::BTreeMap;
use tracing::error;
use tracing::info;

pub trait DriftTypeTrait {
    fn from_str(value: &str) -> DriftType;
}

impl DriftTypeTrait for DriftType {
    fn from_str(value: &str) -> DriftType {
        match value {
            "SPC" => DriftType::SPC,
            "PSI" => DriftType::PSI,
            _ => DriftType::NONE,
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

    pub fn profile_type(&self) -> String {
        match self {
            DriftProfile::SpcDriftProfile(_) => DriftType::SPC.value(),
        }
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
    pub async fn process_task(
        &mut self,
        profile: DriftProfile,
        previous_run: NaiveDateTime,
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<Option<Vec<BTreeMap<String, String>>>, anyhow::Error> {
        // match Drifter enum
        profile
            .get_drifter(
                name.to_string(),
                repository.to_string(),
                version.to_string(),
            )
            .check_for_alerts(&self.db_client, previous_run)
            .await
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

        let profile =
            DriftProfile::from_type(DriftType::from_str(&task.profile_type), task.profile)
                .context("error converting drift profile to DriftProfile");

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
