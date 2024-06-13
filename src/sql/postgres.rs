use crate::sql::query::{GetBinnedFeatureValuesParams, GetFeaturesParams, InsertParams, Queries};
use crate::sql::schema::{DriftRecord, FeatureResult, QueryResult};
use anyhow::*;
use futures::future::join_all;
use sqlx::Row;
use sqlx::{
    postgres::{PgPoolOptions, PgQueryResult, PgRow},
    Pool, Postgres,
};
use std::collections::HashMap;
use std::result::Result::Ok;

use tracing::{error, info};
pub struct PostgresClient {
    pub pool: Pool<Postgres>,
    qualified_table_name: String,
}

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
}

impl PostgresClient {
    // Create a new instance of PostgresClient
    pub async fn new() -> Result<Self, Error> {
        let database_url =
            std::env::var("DATABASE_URL").with_context(|| "DATABASE_URL must be set")?;

        // get max connections from env or set to 10
        let max_connections = std::env::var("MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .expect("MAX_CONNECTIONS must be a number");

        let pool = match PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(&database_url)
            .await
        {
            Ok(pool) => {
                info!("✅ Successfully connected to database");
                pool
            }
            Err(err) => {
                error!("🔥 Failed to connect to database {:?}", err);
                std::process::exit(1);
            }
        };

        Ok(Self {
            pool,
            qualified_table_name: "scouter.drift".to_string(),
        })
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
        record: DriftRecord,
    ) -> Result<PgQueryResult, anyhow::Error> {
        let query = Queries::InsertDriftRecord.get_query();

        let params = InsertParams {
            table: self.qualified_table_name.to_string(),
            service_name: record.service_name,
            feature: record.feature,
            value: record.value.to_string(),
            version: record.version,
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

    // Queries the database for all features under a service
    // Private method that'll be used to run drift retrieval in parallel
    async fn get_service_features(
        &self,
        service_name: &str,
        version: &str,
    ) -> Result<Vec<String>, anyhow::Error> {
        let query = Queries::GetFeatures.get_query();

        let params = GetFeaturesParams {
            table: self.qualified_table_name.to_string(),
            service_name: service_name.to_string(),
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
        bin: &f64,
        feature: String,
        version: &str,
        time_window: &i32,
        service_name: &str,
    ) -> Result<Vec<PgRow>, anyhow::Error> {
        let query = Queries::GetBinnedFeatureValues.get_query();

        let params = GetBinnedFeatureValuesParams {
            table: self.qualified_table_name.to_string(),
            service_name: service_name.to_string(),
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
    // * `service_name` - The name of the service to query drift records for
    // * `feature` - The name of the feature to query drift records for
    // * `aggregation` - The aggregation to use for the query
    // * `time_window` - The time window to query drift records for
    //
    // # Returns
    //
    // * A vector of drift records
    pub async fn read_drift_records(
        &self,
        service_name: &str,
        version: &str,
        max_data_points: &i32,
        time_window: &i32,
    ) -> Result<QueryResult, anyhow::Error> {
        // get features
        let features = self.get_service_features(service_name, version).await?;

        let bin = *time_window as f64 / *max_data_points as f64;

        let async_queries = features
            .iter()
            .map(|feature| {
                self.run_feature_query(
                    &bin,
                    feature.to_string(),
                    version,
                    time_window,
                    service_name,
                )
            })
            .collect::<Vec<_>>();

        let query_results = join_all(async_queries).await;

        // parse results
        let mut query_result = QueryResult {
            features: HashMap::new(),
        };

        for data in query_results {
            match data {
                Ok(data) => {
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

    pub async fn raw_query(&self, query: &str) -> Result<Vec<PgRow>, anyhow::Error> {
        let result = sqlx::raw_sql(query).fetch_all(&self.pool).await;

        match result {
            Ok(result) => {
                // pretty print
                Ok(result)
            }
            Err(e) => {
                error!("Failed to run query: {:?}", e);
                return Err(anyhow!("Failed to run query: {:?}", e));
            }
        }
    }
}

// integration tests
#[cfg(test)]
mod tests {

    use super::*;
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_client() {
        env::set_var(
            "DATABASE_URL",
            "postgresql://postgres:admin@localhost:5432/monitor?",
        );

        PostgresClient::new().await.expect("error");
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
