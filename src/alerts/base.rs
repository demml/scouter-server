use crate::alerts::spc::drift::SpcDrifter;
use crate::alerts::types::Drifter;
use crate::api::schema::ServiceInfo;
use crate::sql::postgres::PostgresClient;

use chrono::NaiveDateTime;
use scouter::core::drift::base::DriftProfile;
use scouter::core::drift::base::DriftType;
use std::collections::BTreeMap;
use std::result::Result;
use std::result::Result::Ok;
use std::str::FromStr;
use tracing::error;
use tracing::info;
pub trait GetDrifter {
    fn get_drifter(&self) -> Drifter;
}

impl GetDrifter for DriftProfile {
    /// Get a Drifter for processing drift profile tasks
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the drift profile
    /// * `repository` - Repository of the drift profile
    /// * `version` - Version of the drift profile
    ///
    /// # Returns
    ///
    /// * `Drifter` - Drifter enum
    fn get_drifter(&self) -> Drifter {
        match self {
            DriftProfile::SpcDriftProfile(profile) => {
                Drifter::SpcDrifter(SpcDrifter::new(profile.clone()))
            }
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
    ) -> Result<Option<Vec<BTreeMap<String, String>>>, anyhow::Error> {
        // match Drifter enum
        profile
            .get_drifter()
            .check_for_alerts(&self.db_client, previous_run)
            .await
    }

    /// Execute single drift computation and alerting
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result of drift computation and alerting
    pub async fn poll_for_tasks(&mut self) -> Result<(), anyhow::Error> {
        let mut transaction = self.db_client.pool.begin().await?;

        // this will pull a drift profile from the db
        let task = match PostgresClient::get_drift_profile_task(&mut transaction).await {
            Ok(task) => task,
            Err(e) => {
                error!("Error getting drift profile task: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                return Ok(());
            }
        };

        let Some(task) = task else {
            transaction.commit().await?;
            info!("No triggered schedules found in db. Sleeping for 10 seconds");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            return Ok(());
        };

        let service_info = ServiceInfo {
            repository: task.repository.clone(),
            name: task.name.clone(),
            version: task.version.clone(),
        };

        // match drift_type
        match DriftType::from_str(&task.drift_type) {
            // match drift_profile
            Ok(drift_type) => match DriftProfile::from_str(drift_type, task.profile) {
                // process drift profile task
                Ok(profile) => match self.process_task(profile, task.previous_run).await {
                    // check for alerts
                    Ok(alerts) => {
                        info!("Drift task processed successfully");

                        if let Some(alerts) = alerts {
                            // insert each task into db
                            for alert in alerts {
                                if let Err(e) = self
                                    .db_client
                                    .insert_drift_alert(
                                        &service_info,
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
                },
                Err(e) => {
                    error!("Error converting drift profile: {:?}", e);
                }
            },
            Err(e) => {
                error!("Error converting drift type: {:?}", e);
            }
        }

        if let Err(e) = PostgresClient::update_drift_profile_run_dates(
            &mut transaction,
            &service_info,
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
