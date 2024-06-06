use crate::schema::DriftRecord;
use anyhow::*;
use sqlx::Row;
use sqlx::{
    postgres::{PgPoolOptions, PgQueryResult},
    Pool, Postgres,
};
use std::result::Result::Ok;
use tracing::{error, info};

pub struct PostgresClient {
    pool: Pool<Postgres>,
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

        Ok(Self { pool })
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
        table_name: &str,
    ) -> Result<PgQueryResult, sqlx::Error> {
        let query = format!(
            "INSERT INTO (service_name,feature,value,version) VALUES ('{}', '{}', {}, '{}');",
            record.service_name, record.feature, record.value, record.version
        );
        let query_result: std::prelude::v1::Result<sqlx::postgres::PgQueryResult, sqlx::Error> =
            sqlx::raw_sql(query.as_str()).execute(&self.pool).await;

        query_result
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
            service_name: "test".to_string(),
            feature: "test".to_string(),
            value: 1.0,
            version: "1.0.0".to_string(),
        };

        let result = sqlx::raw_sql("SELECT * FROM pg_catalog.pg_tables;")
            .fetch_all(&client.pool)
            .await
            .unwrap();

        for row in result {
            let table_name: String = row.get("tablename");
            println!("{:?}", table_name);
        }

        client
            .insert_drift_record(record, "scouter.drift")
            .await
            .unwrap();

        // assert record was written

        // test reading record
        let result = sqlx::raw_sql("SELECT * FROM scouter.drift LIMIT 1")
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

            assert_eq!(record.service_name, "test");
            assert_eq!(record.feature, "test");
            assert_eq!(record.value, 1.0);
            assert_eq!(record.version, "1.0.0");
        }
    }
}
