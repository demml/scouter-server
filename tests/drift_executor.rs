use chrono::NaiveDateTime;
use scouter_server::alerts::drift::DriftExecutor;
use scouter_server::sql::postgres::PostgresClient;
use sqlx::Row;
mod test_utils;

#[tokio::test]
async fn test_drift_executor() {
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    // populate the database
    let populate_script = include_str!("scripts/populate.sql");
    sqlx::raw_sql(populate_script).execute(&pool).await.unwrap();
    let mut drift_executor = DriftExecutor::new(db_client.clone());

    // get current next run
    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_profile
        WHERE name = 'test_app'
        AND repository = 'statworld'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let curr_next_run: NaiveDateTime = result[0].get("next_run");
    drift_executor.execute().await.unwrap();

    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_profile
        WHERE name = 'test_app'
        AND repository = 'statworld'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(result.len(), 1);

    let previous_run: NaiveDateTime = result[0].get("previous_run");

    // assert next run from before computing drift is now the previous run
    assert_eq!(previous_run, curr_next_run);

    test_utils::teardown().await.unwrap();
}
