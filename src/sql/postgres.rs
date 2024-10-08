use crate::api::schema::{
    DriftAlertRequest, DriftRequest, ObservabilityMetricRequest, ProfileStatusRequest, ServiceInfo,
};
use crate::sql::query::Queries;
use crate::sql::schema::{
    AlertResult, FeatureResult, ObservabilityResult, QueryResult, SpcFeatureResult, TaskRequest,
};
use anyhow::*;
use chrono::Utc;
use cron::Schedule;
use futures::future::join_all;
use include_dir::{include_dir, Dir};
use scouter::core::drift::base::DriftProfile;
use scouter::core::drift::spc::types::SpcServerRecord;
use scouter::core::observe::observer::ObservabilityMetrics;
use serde_json::Value;
use sqlx::{
    postgres::{PgQueryResult, PgRow},
    Pool, Postgres, Row, Transaction,
};
use std::collections::BTreeMap;
use std::result::Result::Ok;
use std::str::FromStr;
use tracing::{error, warn};

static _MIGRATIONS: Dir = include_dir!("migrations");

pub enum TimeInterval {
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    ThreeHours,
    SixHours,
    TwelveHours,
    TwentyFourHours,
    TwoDays,
    FiveDays,
}

impl TimeInterval {
    pub fn to_minutes(&self) -> i32 {
        match self {
            TimeInterval::FiveMinutes => 5,
            TimeInterval::FifteenMinutes => 15,
            TimeInterval::ThirtyMinutes => 30,
            TimeInterval::OneHour => 60,
            TimeInterval::ThreeHours => 180,
            TimeInterval::SixHours => 360,
            TimeInterval::TwelveHours => 720,
            TimeInterval::TwentyFourHours => 1440,
            TimeInterval::TwoDays => 2880,
            TimeInterval::FiveDays => 7200,
        }
    }

    pub fn from_string(time_window: &str) -> TimeInterval {
        match time_window {
            "5minute" => TimeInterval::FiveMinutes,
            "15minute" => TimeInterval::FifteenMinutes,
            "30minute" => TimeInterval::ThirtyMinutes,
            "1hour" => TimeInterval::OneHour,
            "3hour" => TimeInterval::ThreeHours,
            "6hour" => TimeInterval::SixHours,
            "12hour" => TimeInterval::TwelveHours,
            "24hour" => TimeInterval::TwentyFourHours,
            "2day" => TimeInterval::TwoDays,
            "5day" => TimeInterval::FiveDays,
            _ => TimeInterval::SixHours,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PostgresClient {
    pub pool: Pool<Postgres>,
}

impl PostgresClient {
    // Create a new instance of PostgresClient
    pub fn new(pool: Pool<Postgres>) -> Result<Self, anyhow::Error> {
        // get database url from env or use the provided one

        Ok(Self { pool })
    }

    // Inserts a drift alert into the database
    //
    // # Arguments
    //
    // * `name` - The name of the service to insert the alert for
    // * `repository` - The name of the repository to insert the alert for
    // * `version` - The version of the service to insert the alert for
    // * `alert` - The alert to insert into the database
    //
    pub async fn insert_drift_alert(
        &self,
        service_info: &ServiceInfo,
        feature: &str,
        alert: &BTreeMap<String, String>,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftAlert.get_query();

        let query_result = sqlx::query(&query.sql)
            .bind(&service_info.name)
            .bind(&service_info.repository)
            .bind(&service_info.version)
            .bind(feature)
            .bind(serde_json::to_value(alert).unwrap())
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to insert alert into database");

        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert alert into database: {:?}", e);
                Err(anyhow!("Failed to insert alert into database: {:?}", e))
            }
        }
    }

    pub async fn get_drift_alerts(
        &self,
        params: &DriftAlertRequest,
    ) -> Result<Vec<AlertResult>, anyhow::Error> {
        let active = params.active.unwrap_or(false);

        let query = Queries::GetDriftAlerts.get_query();

        // check if active
        let built_query = if active {
            format!("{} AND status = 'active'", query.sql)
        } else {
            query.sql
        };

        // check if limit timestamp is provided
        let built_query = if params.limit_timestamp.is_some() {
            format!(
                "{} AND created_at >= '{}' ORDER BY created_at DESC",
                built_query,
                params.limit_timestamp.as_ref().unwrap()
            )
        } else {
            format!("{} ORDER BY created_at DESC", built_query)
        };

        // check if limit is provided
        let built_query = if params.limit.is_some() {
            format!("{} LIMIT {};", built_query, params.limit.unwrap())
        } else {
            format!("{};", built_query)
        };

        let result: Result<Vec<AlertResult>, sqlx::Error> = sqlx::query_as(&built_query)
            .bind(&params.version)
            .bind(&params.name)
            .bind(&params.repository)
            .fetch_all(&self.pool)
            .await;

        match result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to get alerts from database: {:?}", e);
                Err(anyhow!("Failed to get alerts from database: {:?}", e))
            }
        }
    }

    // Inserts a drift record into the database
    //
    // # Arguments
    //
    // * `record` - A drift record to insert into the database
    // * `table_name` - The name of the table to insert the record into
    //
    pub async fn insert_spc_drift_record(
        &self,
        record: &SpcServerRecord,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftRecord.get_query();

        let query_result = sqlx::query(&query.sql)
            .bind(record.created_at)
            .bind(&record.name)
            .bind(&record.repository)
            .bind(&record.version)
            .bind(&record.feature)
            .bind(record.value)
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to insert alert into database");

        //drop params
        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert record into database: {:?}", e);
                Err(anyhow!("Failed to insert record into database: {:?}", e))
            }
        }
    }

    // Inserts a drift record into the database
    //
    // # Arguments
    //
    // * `record` - A drift record to insert into the database
    // * `table_name` - The name of the table to insert the record into
    //
    pub async fn insert_observability_record(
        &self,
        record: &ObservabilityMetrics,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertObservabilityRecord.get_query();
        let route_metrics = serde_json::to_value(&record.route_metrics).map_err(|e| {
            error!("Failed to serialize route metrics: {:?}", e);
            anyhow!("Failed to serialize route metrics: {:?}", e)
        })?;

        let query_result = sqlx::query(&query.sql)
            .bind(&record.repository)
            .bind(&record.name)
            .bind(&record.version)
            .bind(record.request_count)
            .bind(record.error_count)
            .bind(route_metrics)
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to insert observability metrics into database");

        //drop params
        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(
                    "Failed to insert observability record into database: {:?}",
                    e
                );
                Err(anyhow!(
                    "Failed to insert observability record into database: {:?}",
                    e
                ))
            }
        }
    }

    pub async fn insert_drift_profile(
        &self,
        drift_profile: &DriftProfile,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftProfile.get_query();
        let base_args = drift_profile.get_base_args();

        let schedule = Schedule::from_str(&base_args.schedule)
            .with_context(|| format!("Failed to parse cron expression: {}", base_args.schedule))?;

        let next_run = schedule.upcoming(Utc).take(1).next().with_context(|| {
            format!(
                "Failed to get next run time for cron expression: {}",
                base_args.schedule
            )
        })?;

        let query_result = sqlx::query(&query.sql)
            .bind(base_args.name)
            .bind(base_args.repository)
            .bind(base_args.version)
            .bind(base_args.scouter_version)
            .bind(drift_profile.to_value())
            .bind(base_args.drift_type.value())
            .bind(false)
            .bind(base_args.schedule)
            .bind(next_run.naive_utc())
            .bind(next_run.naive_utc())
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to insert profile into database");

        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert record into database: {:?}", e);
                Err(anyhow!("Failed to insert record into database: {:?}", e))
            }
        }
    }

    pub async fn update_drift_profile(
        &self,
        drift_profile: &DriftProfile,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::UpdateDriftProfile.get_query();
        let base_args = drift_profile.get_base_args();

        let query_result = sqlx::query(&query.sql)
            .bind(drift_profile.to_value())
            .bind(base_args.drift_type.value())
            .bind(base_args.name)
            .bind(base_args.repository)
            .bind(base_args.version)
            .execute(&self.pool)
            .await
            .with_context(|| "Failed to insert profile into database");

        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to update data profile: {:?}", e);
                Err(anyhow!("Failed to update data profile: {:?}", e))
            }
        }
    }

    pub async fn get_drift_profile(
        &self,
        params: &ServiceInfo,
    ) -> Result<Option<Value>, anyhow::Error> {
        let query = Queries::GetDriftProfile.get_query();

        let result = sqlx::query(&query.sql)
            .bind(&params.name)
            .bind(&params.repository)
            .bind(&params.version)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| "Failed to get drift profile from database")?;

        match result {
            Some(result) => {
                let profile: Value = result.get("profile");
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }

    pub async fn get_drift_profile_task(
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<TaskRequest>, Error> {
        let query = Queries::GetDriftTask.get_query();
        let result: Result<Option<TaskRequest>, sqlx::Error> = sqlx::query_as(&query.sql)
            .fetch_optional(&mut **transaction)
            .await;

        result.map_err(|e| {
            error!("Failed to get drift task from database: {:?}", e);
            anyhow!("Failed to get drift task from database: {:?}", e)
        })
    }

    pub async fn update_drift_profile_run_dates(
        transaction: &mut Transaction<'_, Postgres>,
        service_info: &ServiceInfo,
        schedule: &str,
    ) -> Result<(), Error> {
        let query = Queries::UpdateDriftProfileRunDates.get_query();

        let schedule = Schedule::from_str(schedule)
            .with_context(|| format!("Failed to parse cron expression: {}", schedule))?;

        let next_run = schedule.upcoming(Utc).take(1).next().with_context(|| {
            format!(
                "Failed to get next run time for cron expression: {}",
                schedule
            )
        })?;

        let query_result = sqlx::query(&query.sql)
            .bind(next_run.naive_utc())
            .bind(&service_info.name)
            .bind(&service_info.repository)
            .bind(&service_info.version)
            .execute(&mut **transaction)
            .await;

        match query_result {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(
                "Failed to update drift profile run dates in database: {:?}",
                e
            )),
        }
    }

    // Queries the database for all features under a service
    // Private method that'll be used to run drift retrieval in parallel
    async fn get_features(&self, service_info: &ServiceInfo) -> Result<Vec<String>, anyhow::Error> {
        let query = Queries::GetFeatures.get_query();

        sqlx::query(&query.sql)
            .bind(&service_info.name)
            .bind(&service_info.repository)
            .bind(&service_info.version)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to get features from database: {:?}", e);
                anyhow!("Failed to get features from database: {:?}", e)
            })
            .map(|result| {
                result
                    .iter()
                    .map(|row| row.get("feature"))
                    .collect::<Vec<String>>()
            })
    }

    async fn run_spc_feature_query(
        &self,
        feature: &str,
        service_info: &ServiceInfo,
        limit_timestamp: &str,
    ) -> Result<SpcFeatureResult, anyhow::Error> {
        let query = Queries::GetFeatureValues.get_query();

        let feature_values: Result<SpcFeatureResult, anyhow::Error> = sqlx::query_as(&query.sql)
            .bind(limit_timestamp)
            .bind(&service_info.name)
            .bind(&service_info.repository)
            .bind(&service_info.version)
            .bind(feature)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to run query: {:?}", e);
                anyhow!("Failed to run query: {:?}", e)
            });

        feature_values
    }

    pub async fn get_binned_observability_metrics(
        &self,
        params: &ObservabilityMetricRequest,
    ) -> Result<Vec<ObservabilityResult>, anyhow::Error> {
        let query = Queries::GetBinnedObservabilityMetrics.get_query();

        let time_window = TimeInterval::from_string(&params.time_window).to_minutes();

        let bin = time_window as f64 / params.max_data_points as f64;

        let observability_metrics: Result<Vec<ObservabilityResult>, sqlx::Error> =
            sqlx::query_as(&query.sql)
                .bind(bin)
                .bind(time_window)
                .bind(&params.name)
                .bind(&params.repository)
                .bind(&params.version)
                .fetch_all(&self.pool)
                .await;

        observability_metrics.map_err(|e| {
            error!("Failed to run query: {:?}", e);
            anyhow!("Failed to run query: {:?}", e)
        })
    }

    async fn get_spc_binned_feature_values(
        &self,
        bin: &f64,
        feature: String,
        version: &str,
        time_window: &i32,
        repository: &str,
        name: &str,
    ) -> Result<SpcFeatureResult, anyhow::Error> {
        let query = Queries::GetBinnedFeatureValues.get_query();

        let binned: Result<SpcFeatureResult, sqlx::Error> = sqlx::query_as(&query.sql)
            .bind(bin)
            .bind(time_window)
            .bind(name)
            .bind(repository)
            .bind(version)
            .bind(feature)
            .fetch_one(&self.pool)
            .await;

        binned.map_err(|e| {
            error!("Failed to run query: {:?}", e);
            anyhow!("Failed to run query: {:?}", e)
        })
    }

    // Queries the database for drift records based on a time window and aggregation
    //
    // # Arguments
    //
    // * `name` - The name of the service to query drift records for
    // * `repository` - The name of the repository to query drift records for
    // * `feature` - The name of the feature to query drift records for
    // * `aggregation` - The aggregation to use for the query
    // * `time_window` - The time window to query drift records for
    //
    // # Returns
    //
    // * A vector of drift records
    pub async fn get_binned_drift_records(
        &self,
        params: &DriftRequest,
    ) -> Result<QueryResult, anyhow::Error> {
        let service_info = ServiceInfo {
            repository: params.repository.clone(),
            name: params.name.clone(),
            version: params.version.clone(),
        };
        // get features
        let features = self.get_features(&service_info).await?;

        let time_window = TimeInterval::from_string(&params.time_window).to_minutes();

        let bin = time_window as f64 / params.max_data_points as f64;

        let async_queries = features
            .iter()
            .map(|feature| {
                self.get_spc_binned_feature_values(
                    &bin,
                    feature.to_string(),
                    &params.version,
                    &time_window,
                    &params.repository,
                    &params.name,
                )
            })
            .collect::<Vec<_>>();

        let query_results = join_all(async_queries).await;

        // parse results
        let mut query_result = QueryResult {
            features: BTreeMap::new(),
        };

        query_results.iter().for_each(|result| match result {
            Ok(result) => {
                query_result.features.insert(
                    result.feature.clone(),
                    FeatureResult {
                        created_at: result.created_at.clone(),
                        values: result.values.clone(),
                    },
                );
            }
            Err(e) => {
                error!("Failed to run query: {:?}", e);
            }
        });

        Ok(query_result)
    }

    pub async fn get_drift_records(
        &self,
        service_info: &ServiceInfo,
        limit_timestamp: &str,
        features_to_monitor: &[String],
    ) -> Result<QueryResult, anyhow::Error> {
        let mut features = self.get_features(service_info).await?;

        if !features_to_monitor.is_empty() {
            features.retain(|feature| features_to_monitor.contains(feature));
        }

        let query_results = join_all(
            features
                .iter()
                .map(|feature| self.run_spc_feature_query(feature, service_info, limit_timestamp))
                .collect::<Vec<_>>(),
        )
        .await;

        let mut query_result = QueryResult {
            features: BTreeMap::new(),
        };

        let feature_sizes = query_results
            .iter()
            .map(|result| match result {
                Ok(result) => result.values.len(),
                Err(_) => 0,
            })
            .collect::<Vec<_>>();

        // check if all feature values have the same length
        // log a warning if they don't
        if feature_sizes.windows(2).any(|w| w[0] != w[1]) {
            warn!(
                    "Feature values have different lengths for drift profile: {}/{}/{}, Timestamp: {:?}, feature sizes: {:?}",
                    service_info.repository, service_info.name, service_info.version, limit_timestamp, feature_sizes
                );
        }

        // Get smallest non-zero feature size
        let min_feature_size = feature_sizes
            .iter()
            .filter(|size| **size > 0)
            .min()
            .unwrap_or(&0);

        for data in query_results {
            match data {
                Ok(data) if !data.values.is_empty() => {
                    query_result.features.insert(
                        data.feature.clone(),
                        FeatureResult {
                            values: data.values[..*min_feature_size].to_vec(),
                            created_at: data.created_at[..*min_feature_size].to_vec(),
                        },
                    );
                }
                Ok(_) => continue,
                Err(e) => {
                    error!("Failed to run query: {:?}", e);
                    return Err(anyhow!("Failed to run query: {:?}", e));
                }
            }
        }
        Ok(query_result)
    }

    #[allow(dead_code)]
    pub async fn raw_query(&self, query: &str) -> Result<Vec<PgRow>, anyhow::Error> {
        let result = sqlx::raw_sql(query).fetch_all(&self.pool).await;

        match result {
            Ok(result) => {
                // pretty print
                Ok(result)
            }
            Err(e) => {
                error!("Failed to run query: {:?}", e);
                Err(anyhow!("Failed to run query: {:?}", e))
            }
        }
    }

    pub async fn update_drift_profile_status(
        &self,
        params: &ProfileStatusRequest,
    ) -> Result<(), anyhow::Error> {
        let query = Queries::UpdateDriftProfileStatus.get_query();

        let query_result = sqlx::query(&query.sql)
            .bind(params.active)
            .bind(&params.name)
            .bind(&params.repository)
            .bind(&params.version)
            .execute(&self.pool)
            .await;

        match query_result {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to update drift profile status: {:?}", e);
                Err(anyhow!("Failed to update drift profile status: {:?}", e))
            }
        }
    }
}

// integration tests
#[cfg(test)]
mod tests {

    use crate::api::setup::create_db_pool;

    use super::*;
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_client() {
        unsafe {
            env::set_var(
                "DATABASE_URL",
                "postgresql://postgres:admin@localhost:5432/monitor?",
            );
        }

        let pool = create_db_pool(None)
            .await
            .with_context(|| "Failed to create Postgres client")
            .unwrap();
        PostgresClient::new(pool).unwrap();
    }

    #[test]
    fn test_time_interval() {
        assert_eq!(TimeInterval::FiveMinutes.to_minutes(), 5);
        assert_eq!(TimeInterval::FifteenMinutes.to_minutes(), 15);
        assert_eq!(TimeInterval::ThirtyMinutes.to_minutes(), 30);
        assert_eq!(TimeInterval::OneHour.to_minutes(), 60);
        assert_eq!(TimeInterval::ThreeHours.to_minutes(), 180);
        assert_eq!(TimeInterval::SixHours.to_minutes(), 360);
        assert_eq!(TimeInterval::TwelveHours.to_minutes(), 720);
        assert_eq!(TimeInterval::TwentyFourHours.to_minutes(), 1440);
        assert_eq!(TimeInterval::TwoDays.to_minutes(), 2880);
        assert_eq!(TimeInterval::FiveDays.to_minutes(), 7200);
    }
}
