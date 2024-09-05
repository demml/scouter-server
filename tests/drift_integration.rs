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
async fn test_drift_executor() {
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
    let schedule: String = profile.get("schedule");

    let drift_array = drift_executor
        .compute_drift(&drift_profile, &previous_run)
        .await
        .unwrap();

    assert_eq!(drift_array.shape(), [10, 3] as [usize; 2]);

    println!("{:?}", drift_array);

    let keys = drift_profile.features.keys();
    println!("{:?}", keys);

    let alerts = generate_alerts(
        &drift_array.view(),
        drift_profile.features.keys().cloned().collect(),
        drift_profile.config.alert_config.alert_rule,
    )
    .unwrap();

    println!("{:?}", alerts);

    assert_eq!(
        alerts.features.get("col_2").unwrap().alerts[0].zone,
        "center"
    );

    ConsoleAlertDispatcher
        .process_alerts(
            &alerts,
            &drift_profile.config.repository,
            &drift_profile.config.name,
        )
        .await
        .unwrap();

    transaction.commit().await.unwrap();
    test_utils::teardown().await.unwrap();
}
