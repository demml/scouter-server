use chrono::NaiveDateTime;
use scouter::core::alert::generate_alerts;
use scouter::utils::types::DriftProfile;
use scouter_server::alerts::dispatch::ConsoleAlertDispatcher;
use scouter_server::alerts::dispatch::Dispatch;
use scouter_server::alerts::drift::DriftExecutor;
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

    let drift_executor = DriftExecutor::new(db_client.clone());

    let mut transaction: sqlx::Transaction<Postgres> = db_client.pool.begin().await.unwrap();
    let profile = PostgresClient::get_drift_profile(&mut transaction)
        .await
        .unwrap();

    // asert profile is not none
    assert!(profile.is_some());

    let profile = profile.unwrap();

    let drift_profile = serde_json::from_value::<DriftProfile>(profile.get("profile")).unwrap();

    // assert drift_profile.config.name is "test_app"
    assert_eq!(drift_profile.config.name, "test_app");
    assert_eq!(drift_profile.config.repository, "statworld");

    // switch back to previous run
    let previous_run: NaiveDateTime = profile.get("previous_run");
    let name: String = profile.get("name");
    let repository: String = profile.get("repository");
    let version: String = profile.get("version");

    let (drift_array, sample_array, keys) = drift_executor
        .compute_drift(&drift_profile, &previous_run, &name, &repository, &version)
        .await
        .unwrap();

    assert_eq!(drift_array.shape(), [10, 3] as [usize; 2]);

    let alert_rule = drift_profile.config.alert_config.alert_rule.clone();
    let (alerts, _has_alert) = generate_alerts(
        &drift_array.view(),
        &sample_array.view(),
        &keys,
        &alert_rule,
    )
    .unwrap();

    assert_eq!(
        alerts.features.get("col_3").unwrap().alerts[0].zone,
        "Zone 4"
    );

    let dispatcher = ConsoleAlertDispatcher::new(&name, &repository, &version);
    dispatcher.process_alerts(&alerts).await.unwrap();

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

    test_utils::teardown().await.unwrap();
}
