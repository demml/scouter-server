use crate::sql::query::{
    GetBinnedFeatureValuesParams, GetDriftAlertsParams, GetDriftProfileParams,
    GetDriftProfileTaskParams, GetFeatureValuesParams, GetFeaturesParams, InsertDriftAlertParams,
    InsertDriftProfileParams, InsertParams, Queries, UpdateDriftProfileParams,
    UpdateDriftProfileRunDatesParams, UpdateDriftProfileStatusParams, DRIFT_ALERT_TABLE,
    DRIFT_PROFILE_TABLE, DRIFT_TABLE,
};
use crate::sql::schema::{
    AlertResult, DriftRecord, FeatureDistribution, FeatureResult, QueryResult,
};
use anyhow::*;
use chrono::Utc;
use cron::Schedule;
use futures::future::join_all;
use include_dir::{include_dir, Dir};
use scouter::utils::types::DriftProfile;
use serde_json::Value;
use sqlx::{
    postgres::{PgQueryResult, PgRow},
    Pool, Postgres, QueryBuilder, Row, Transaction,
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
    drift_table_name: String,
    profile_table_name: String,
    alert_table_name: String,
}

impl PostgresClient {
    // Create a new instance of PostgresClient
    pub fn new(pool: Pool<Postgres>) -> Result<Self, anyhow::Error> {
        // get database url from env or use the provided one

        Ok(Self {
            pool,
            drift_table_name: DRIFT_TABLE.to_string(),
            profile_table_name: DRIFT_PROFILE_TABLE.to_string(),
            alert_table_name: DRIFT_ALERT_TABLE.to_string(),
        })
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
        name: &str,
        repository: &str,
        version: &str,
        feature: &str,
        alert: &BTreeMap<String, String>,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftAlert.get_query();

        let params = InsertDriftAlertParams {
            table: self.alert_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
            feature: feature.to_string(),
            alert: serde_json::to_string(&alert).unwrap(),
        };

        let query_result: std::prelude::v1::Result<sqlx::postgres::PgQueryResult, sqlx::Error> =
            sqlx::raw_sql(query.format(&params).as_str())
                .execute(&self.pool)
                .await;

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
        name: &str,
        repository: &str,
        version: &str,
        limit_timestamp: Option<&str>,
    ) -> Result<Vec<AlertResult>, anyhow::Error> {
        let query = Queries::GetDriftAlerts.get_query();

        let params = GetDriftAlertsParams {
            table: self.alert_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
        };

        let mut formatted_query = query.format(&params);

        if limit_timestamp.is_some() {
            let limit_timestamp = limit_timestamp.unwrap();
            formatted_query = format!(
                "{} AND created_at >= '{}' ORDER BY created_at DESC LIMIT 50;",
                formatted_query, limit_timestamp
            );
        } else {
            formatted_query = format!("{} ORDER BY created_at DESC LIMIT 50;", formatted_query);
        }

        let result = sqlx::raw_sql(formatted_query.as_str())
            .fetch_all(&self.pool)
            .await;

        match result {
            Ok(result) => {
                let mut results = Vec::new();

                result.iter().for_each(|row| {
                    let alerts = serde_json::from_value::<BTreeMap<String, String>>(
                        row.get("alert"),
                    )
                    .with_context(|| {
                        "error converting postgres jsonb profile to struct type DriftProfile"
                    });

                    match alerts {
                        Ok(alerts) => {
                            let alert = AlertResult {
                                name: row.get("name"),
                                repository: row.get("repository"),
                                version: row.get("version"),
                                created_at: row.get("created_at"),
                                feature: row.get("feature"),
                                alerts,
                                id: row.get("id"),
                                status: row.get("status"),
                            };
                            results.push(alert);
                        }
                        Err(e) => {
                            error!("Failed to get alerts from database: {:?}", e);
                        }
                    }
                });

                Ok(results)
            }
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
    pub async fn insert_drift_record(
        &self,
        record: &DriftRecord,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftRecord.get_query();

        let params = InsertParams {
            table: self.drift_table_name.to_string(),
            created_at: record.created_at,
            name: record.name.clone(),
            repository: record.repository.clone(),
            feature: record.feature.clone(),
            value: record.value.to_string(),
            version: record.version.clone(),
        };

        let query_result: std::prelude::v1::Result<sqlx::postgres::PgQueryResult, sqlx::Error> =
            sqlx::raw_sql(query.format(&params).as_str())
                .execute(&self.pool)
                .await;

        //drop params
        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert record into database: {:?}", e);
                Err(anyhow!("Failed to insert record into database: {:?}", e))
            }
        }
    }

    pub async fn insert_drift_profile(
        &self,
        drift_profile: &DriftProfile,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftProfile.get_query();

        let schedule = Schedule::from_str(&drift_profile.config.alert_config.schedule)
            .with_context(|| {
                format!(
                    "Failed to parse cron expression: {}",
                    &drift_profile.config.alert_config.schedule
                )
            })?;

        let next_run = schedule.upcoming(Utc).take(1).next().with_context(|| {
            format!(
                "Failed to get next run time for cron expression: {}",
                &drift_profile.config.alert_config.schedule
            )
        })?;

        let params = InsertDriftProfileParams {
            table: "scouter.drift_profile".to_string(),
            name: drift_profile.config.name.clone(),
            repository: drift_profile.config.repository.clone(),
            version: drift_profile.config.version.clone(),
            profile: serde_json::to_string(&drift_profile).unwrap(),
            scouter_version: drift_profile.scouter_version.clone(),
            active: false,
            schedule: drift_profile.config.alert_config.schedule.clone(),
            next_run: next_run.naive_utc(),
            previous_run: next_run.naive_utc(),
        };

        let query_result: std::prelude::v1::Result<sqlx::postgres::PgQueryResult, sqlx::Error> =
            sqlx::raw_sql(query.format(&params).as_str())
                .execute(&self.pool)
                .await;

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

        let params = UpdateDriftProfileParams {
            table: "scouter.drift_profile".to_string(),
            name: drift_profile.config.name.clone(),
            repository: drift_profile.config.repository.clone(),
            version: drift_profile.config.version.clone(),
            profile: serde_json::to_string(&drift_profile).unwrap(),
        };

        let query_result: std::prelude::v1::Result<sqlx::postgres::PgQueryResult, sqlx::Error> =
            sqlx::raw_sql(query.format(&params).as_str())
                .execute(&self.pool)
                .await;

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
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<Option<Value>, anyhow::Error> {
        let query = Queries::GetDriftProfile.get_query();

        let params = GetDriftProfileParams {
            table: self.profile_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
        };

        let result = sqlx::query(query.format(&params).as_str())
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
    ) -> Result<Option<PgRow>, Error> {
        let query = Queries::GetDriftTask.get_query();

        let params = GetDriftProfileTaskParams {
            table: "scouter.drift_profile".to_string(),
        };

        let result = sqlx::query(query.format(&params).as_str())
            .fetch_optional(&mut **transaction)
            .await
            .with_context(|| "Failed to get drift profile from database")?;

        Ok(result)
    }

    pub async fn update_drift_profile_run_dates(
        transaction: &mut Transaction<'_, Postgres>,
        name: &str,
        repository: &str,
        version: &str,
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

        let params = UpdateDriftProfileRunDatesParams {
            table: "scouter.drift_profile".to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
            next_run: next_run.naive_utc(),
        };

        let query_result = sqlx::query(query.format(&params).as_str())
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

    //func batch insert drift records
    #[allow(dead_code)]
    pub async fn insert_drift_records(
        &self,
        records: &[DriftRecord],
    ) -> Result<PgQueryResult, anyhow::Error> {
        let insert_statement = format!(
            "INSERT INTO {} (created_at, name, repository, version, feature, value)",
            self.drift_table_name
        );

        let mut query_builder = QueryBuilder::new(insert_statement);

        query_builder.push_values(records.iter(), |mut b, record| {
            b.push_bind(record.created_at)
                .push_bind(&record.name)
                .push_bind(&record.repository)
                .push_bind(&record.version)
                .push_bind(&record.feature)
                .push_bind(record.value);
        });

        let query = query_builder.build();

        let query_result = query.execute(&self.pool).await;

        match query_result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to insert record into database: {:?}", e);
                Err(anyhow!("Failed to insert record into database: {:?}", e))
            }
        }
    }

    // Queries the database for all features under a service
    // Private method that'll be used to run drift retrieval in parallel
    async fn get_features(
        &self,
        name: &str,
        repository: &str,
        version: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let query = Queries::GetFeatures.get_query();

        let params = GetFeaturesParams {
            table: self.drift_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
        };

        let result = sqlx::raw_sql(query.format(&params).as_str())
            .fetch_all(&self.pool)
            .await?;

        let mut features = Vec::new();

        for row in result {
            features.push(row.get("feature"));
        }

        Ok(features)
    }

    async fn run_feature_query(
        &self,
        feature: &str,
        name: &str,
        repository: &str,
        version: &str,
        limit_timestamp: &str,
    ) -> Result<Vec<PgRow>, anyhow::Error> {
        let query = Queries::GetFeatureValues.get_query();

        let params = GetFeatureValuesParams {
            table: self.drift_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
            feature: feature.to_string(),
            limit_timestamp: limit_timestamp.to_string(),
        };

        let result = sqlx::raw_sql(query.format(&params).as_str())
            .fetch_all(&self.pool)
            .await;

        match result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to run query: {:?}", e);
                Err(anyhow!("Failed to run query: {:?}", e))
            }
        }
    }

    async fn run_binned_feature_query(
        &self,
        bin: &f64,
        feature: String,
        version: &str,
        time_window: &i32,
        name: &str,
        repository: &str,
    ) -> Result<Vec<PgRow>, anyhow::Error> {
        let query = Queries::GetBinnedFeatureValues.get_query();

        let params = GetBinnedFeatureValuesParams {
            table: self.drift_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            feature,
            version: version.to_string(),
            time_window: time_window.to_string(),
            bin: bin.to_string(),
        };

        let result = sqlx::raw_sql(query.format(&params).as_str())
            .fetch_all(&self.pool)
            .await;

        match result {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Failed to run query: {:?}", e);
                Err(anyhow!("Failed to run query: {:?}", e))
            }
        }
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
        name: &str,
        repository: &str,
        version: &str,
        max_data_points: &i32,
        time_window: &i32,
        feature: Option<String>,
    ) -> Result<QueryResult, anyhow::Error> {
        // get features
        let mut features = self.get_features(name, repository, version).await?;

        if feature.is_some() {
            let feature = feature.unwrap();
            // filter out features that are not in the list
            features.retain(|f| f == &feature);
        }

        let bin = *time_window as f64 / *max_data_points as f64;

        let async_queries = features
            .iter()
            .map(|feature| {
                self.run_binned_feature_query(
                    &bin,
                    feature.to_string(),
                    version,
                    time_window,
                    name,
                    repository,
                )
            })
            .collect::<Vec<_>>();

        let query_results = join_all(async_queries).await;

        // parse results
        let mut query_result = QueryResult {
            features: BTreeMap::new(),
        };

        for data in query_results {
            match data {
                Ok(data) => {
                    //check if data is empty
                    if data.is_empty() {
                        continue;
                    }

                    let feature_name = data[0].get("feature");
                    let mut created_at = Vec::new();
                    let mut values = Vec::new();

                    for row in data {
                        created_at.push(row.get("created_at"));
                        values.push(row.get("value"));
                    }

                    query_result
                        .features
                        .insert(feature_name, FeatureResult { created_at, values });
                }
                Err(e) => {
                    error!("Failed to run query: {:?}", e);
                    return Err(anyhow!("Failed to run query: {:?}", e));
                }
            }
        }

        Ok(query_result)
    }

    pub async fn get_feature_distribution(
        &self,
        name: &str,
        repository: &str,
        version: &str,
        max_data_points: &i32,
        time_window: &i32,
        feature: &str,
    ) -> Result<FeatureDistribution, anyhow::Error> {
        let query = Queries::GetFeatureDistribution.get_query();

        let bin = *time_window as f64 / *max_data_points as f64;

        let params = GetBinnedFeatureValuesParams {
            table: self.drift_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            feature: feature.to_string(),
            version: version.to_string(),
            time_window: time_window.to_string(),
            bin: bin.to_string(),
        };

        let result: Result<Vec<PgRow>, sqlx::Error> = sqlx::raw_sql(query.format(&params).as_str())
            .fetch_all(&self.pool)
            .await;

        match result {
            Ok(result) => {
                // load to FeatureDistribution

                // check if data is empty
                if result.is_empty() {
                    return Err(anyhow!("No data found for feature distribution"));
                }

                let record = &result[0];

                let feature = FeatureDistribution {
                    name: record.get("name"),
                    repository: record.get("repository"),
                    version: record.get("version"),
                    percentile_10: record.get("percentile_10"),
                    percentile_20: record.get("percentile_20"),
                    percentile_30: record.get("percentile_30"),
                    percentile_40: record.get("percentile_40"),
                    percentile_50: record.get("percentile_50"),
                    percentile_60: record.get("percentile_60"),
                    percentile_70: record.get("percentile_70"),
                    percentile_80: record.get("percentile_80"),
                    percentile_90: record.get("percentile_90"),
                    percentile_100: record.get("percentile_100"),
                    val_10: record.get("val_10"),
                    val_20: record.get("val_20"),
                    val_30: record.get("val_30"),
                    val_40: record.get("val_40"),
                    val_50: record.get("val_50"),
                    val_60: record.get("val_60"),
                    val_70: record.get("val_70"),
                    val_80: record.get("val_80"),
                    val_90: record.get("val_90"),
                    val_100: record.get("val_100"),
                };

                Ok(feature)
            }
            Err(e) => {
                error!("Failed to run query: {:?}", e);
                Err(anyhow!("Failed to run query: {:?}", e))
            }
        }
    }

    pub async fn get_drift_records(
        &self,
        name: &str,
        repository: &str,
        version: &str,
        limit_timestamp: &str,
        features_to_monitor: &[String],
    ) -> Result<QueryResult, anyhow::Error> {
        let mut features = self.get_features(name, repository, version).await?;

        if !features_to_monitor.is_empty() {
            features.retain(|feature| features_to_monitor.contains(feature));
        }

        let async_queries = features
            .iter()
            .map(|feature| {
                self.run_feature_query(feature, name, repository, version, limit_timestamp)
            })
            .collect::<Vec<_>>();

        let query_results = join_all(async_queries).await;

        let mut query_result = QueryResult {
            features: BTreeMap::new(),
        };

        let feature_sizes = query_results
            .iter()
            .map(|result| match result {
                Ok(result) => result.len(),
                Err(_) => 0,
            })
            .collect::<Vec<_>>();

        // check if all feature values have the same length
        // log a warning if they don't
        if !feature_sizes.iter().all(|size| *size == feature_sizes[0]) {
            let msg = format!(
                "Feature values have different lengths for drift profile: {}/{}/{}",
                name, repository, version
            );

            warn!(
                "{}, Timestamp: {:?}, feature sizes: {:?}",
                msg, limit_timestamp, feature_sizes
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
                Ok(data) => {
                    //check if data is empty
                    if data.is_empty() {
                        continue;
                    }

                    let feature_name = data[0].get("feature");
                    let mut created_at = Vec::new();
                    let mut values = Vec::new();

                    data.iter().enumerate().for_each(|(i, row)| {
                        if i < *min_feature_size {
                            created_at.push(row.get("created_at"));
                            values.push(row.get("value"));
                        }
                    });

                    query_result
                        .features
                        .insert(feature_name, FeatureResult { created_at, values });
                }
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
        name: &str,
        repository: &str,
        version: &str,
        active: &bool,
    ) -> Result<(), anyhow::Error> {
        let query = Queries::UpdateDriftProfileStatus.get_query();

        let params = UpdateDriftProfileStatusParams {
            table: self.profile_table_name.to_string(),
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
            active: *active,
        };

        let query_result = sqlx::raw_sql(query.format(&params).as_str())
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
