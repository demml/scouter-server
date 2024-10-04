use chrono::NaiveDateTime;
use scouter::core::dispatch::dispatcher::dispatcher_logic::{ConsoleAlertDispatcher, Dispatch};

use scouter::core::drift::spc::types::SpcDriftProfile;
use scouter_server::alerts::base::DriftExecutor;
use scouter_server::alerts::spc::drift::SpcDrifter;
use scouter_server::sql::postgres::PostgresClient;
use sqlx::{Postgres, Row};
mod test_utils;

#[tokio::test]
async fn test_drift_executor_separate() {
    let pool = test_utils::setup_db(true).await.unwrap();
    let db_client = PostgresClient::new(pool.clone()).unwrap();

    // populate the database
    let populate_script = include_str!("scripts/populate.sql");
    sqlx::raw_sql(populate_script).execute(&pool).await.unwrap();

    let mut transaction: sqlx::Transaction<Postgres> = db_client.pool.begin().await.unwrap();
    let profile = PostgresClient::get_drift_profile_task(&mut transaction)
        .await
        .unwrap();

    // asert profile is not none
    assert!(profile.is_some());

    let profile = profile.unwrap();
    let drift_profile: SpcDriftProfile = serde_json::from_str(&profile.profile).unwrap();

    // assert drift_profile.config.name is "test_app"
    assert_eq!(drift_profile.config.name, "test_app");
    assert_eq!(drift_profile.config.repository, "statworld");

    // switch back to previous run
    let previous_run: NaiveDateTime = profile.previous_run;
    let name: String = profile.name;
    let repository: String = profile.repository;
    let version: String = profile.version;

    let drifter = SpcDrifter::new(drift_profile.clone());

    let (drift_array, keys) = drifter
        .compute_drift(&previous_run, &db_client)
        .await
        .unwrap();

    assert_eq!(drift_array.shape(), [10, 3] as [usize; 2]);

    let alerts = drifter
        .generate_alerts(&drift_array.view(), &keys)
        .await
        .unwrap();

    let task = alerts.unwrap();

    assert_eq!(
        task.alerts.features.get("col_3").unwrap().alerts[0].zone,
        "Zone 4"
    );

    let dispatcher = ConsoleAlertDispatcher::new(&name, &repository, &version);
    dispatcher.process_alerts(&task.alerts).await.unwrap();

    transaction.commit().await.unwrap();
    test_utils::teardown().await.unwrap();
}

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
    drift_executor.poll_for_tasks().await.unwrap();

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

    let result = sqlx::raw_sql(
        r#"
        SELECT * 
        FROM scouter.drift_alerts
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(result.len(), 2);

    test_utils::teardown().await.unwrap();
}
