use crate::schema::{DriftRecord, FeatureResult, QueryResult};
use anyhow::*;
use futures::future::join_all;
use sqlx::Row;
use sqlx::{
    postgres::{PgPoolOptions, PgQueryResult, PgRow},
    Pool, Postgres,
};
use std::collections::HashMap;
use std::result::Result::Ok;
use tokio::task::JoinError;
use tracing::{error, info};
pub struct PostgresClient {
    pool: Pool<Postgres>,
    table_name: String,
    db_schema: String,
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
            table_name: "drift".to_string(),
            db_schema: "scouter".to_string(),
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
    ) -> Result<PgQueryResult, sqlx::Error> {
        let query = format!(
            "INSERT INTO {}.{} 
                (service_name,feature,value,version) 
            VALUES ('{}', '{}', {}, '{}');",
            self.db_schema,
            self.table_name,
            record.service_name,
            record.feature,
            record.value,
            record.version
        );
        let query_result: std::prelude::v1::Result<sqlx::postgres::PgQueryResult, sqlx::Error> =
            sqlx::raw_sql(query.as_str()).execute(&self.pool).await;

        query_result
    }

    // Queries the database for all features under a service
    // Private method that'll be used to run drift retrieval in parallel
    async fn get_service_features(
        &self,
        service_name: &str,
        version: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let query = format!(
            "SELECT DISTINCT feature FROM {}.{} WHERE service_name = '{}' AND version = '{}';",
            self.db_schema, self.table_name, service_name, version
        );

        let result = sqlx::raw_sql(query.as_str()).fetch_all(&self.pool).await?;

        let mut features = Vec::new();

        for row in result {
            features.push(row.get("feature"));
        }

        Ok(features)
    }

    async fn run_feature_query(
        &self,
        bin: &f32,
        feature: String,
        version: &str,
        time_window: &i32,
        service_name: &str,
    ) -> Result<Vec<PgRow>, Error> {
        let subquery = format!(
            "
        SELECT 
        date_bin('{} minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
        service_name,
        feature,
        version,
        value
        from {}.{}
        WHERE 
            created_at > timezone('utc', now()) - interval '{} minutes'
            AND version = '{}
            AND service_name = '{}'
            AND feature = '{}'",
            bin, self.db_schema, self.table_name, time_window, version, service_name, feature
        );

        let query = format!(
            "
        with subquery as ({})
        
        SELECT
            created_at,
            service_name,
            feature,
            version,
            avg(value) as value
        FROM subquery
        GROUP BY 
            created_at,
            service_name,
            feature,
            version
        ORDER BY
            created_at DESC
        ",
            subquery
        );

        let result = sqlx::raw_sql(query.as_str()).fetch_all(&self.pool).await;

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
    ) -> Result<QueryResult, sqlx::Error> {
        // get features
        let features = self.get_service_features(service_name, version).await?;

        let bin = (time_window / max_data_points) as f32;

        let query_vec = features
            .iter()
            .map(|feature| {
                tokio::spawn(async move {
                    self.run_feature_query(
                        &bin,
                        feature.clone(),
                        version,
                        time_window,
                        service_name,
                    )
                    .await
                })
            })
            .collect::<Vec<_>>();

        let query_results = QueryResult {
            features: HashMap::new(),
        };
        // iterate over the results and create a hashmap with keys of created_at and values with values of vec string and vec f64, respectively
        join_all(query_vec).await.iter().for_each(
            |result: &Result<Result<Vec<PgRow>, Error>, JoinError>| match result {
                Ok(result) => {
                    let data: &Vec<PgRow> = result.unwrap().as_ref();
                    let feature_name = data[0].get("feature");
                    let created_at = Vec::new();
                    let values = Vec::new();

                    for row in data {
                        created_at.push(row.get("created_at"));
                        values.push(row.get("value"));
                    }
                    // append to query_results
                    query_results
                        .features
                        .insert(feature_name, FeatureResult { created_at, values });
                }
                Err(e) => error!("Failed to run query: {:?}", e),
            },
        );

        Ok(query_results)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::env;
    use tokio;

    #[tokio::test]
    async fn test_postgres_client() {
        env::set_var(
            "DATABASE_URL",
            "postgresql://postgres:admin@localhost:5432/monitor?",
        );

        let client = PostgresClient::new().await.expect("error");

        // test inserting record
        let record = DriftRecord {
            service_name: "postgres_client".to_string(),
            feature: "test".to_string(),
            value: 1.0,
            version: "1.0.0".to_string(),
        };

        client.insert_drift_record(record).await.unwrap();

        // assert record was written

        // test reading record
        let result = sqlx::raw_sql(
            r#"
            SELECT * 
            FROM scouter.drift  
            WHERE service_name = 'postgres_client'
            LIMIT 1
            "#,
        )
        .fetch_all(&client.pool)
        .await
        .unwrap();

        // iterate over the result and create DriftRecord
        for row in result {
            let record = DriftRecord {
                service_name: row.get("service_name"),
                feature: row.get("feature"),
                value: row.get("value"),
                version: row.get("version"),
            };

            assert_eq!(record.service_name, "postgres_client");
            assert_eq!(record.feature, "test");
            assert_eq!(record.value, 1.0);
            assert_eq!(record.version, "1.0.0");
        }

        // delete all records of service name postgres_client
        sqlx::raw_sql(
            r#"
            DELETE 
            FROM scouter.drift  
            WHERE service_name = 'postgres_client'
            "#,
        )
        .fetch_all(&client.pool)
        .await
        .unwrap();

        // assert record was deleted
        let result = sqlx::raw_sql(
            r#"
            SELECT * 
            FROM scouter.drift  
            WHERE service_name = 'postgres_client'
            "#,
        )
        .fetch_all(&client.pool)
        .await
        .unwrap();

        assert_eq!(result.len(), 0);
    }
}
