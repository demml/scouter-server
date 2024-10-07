use anyhow::Error;
use axum::Router;

use scouter_server::api::route::create_router;
use scouter_server::api::route::AppState;
use scouter_server::api::setup::create_db_pool;
use scouter_server::sql::postgres::PostgresClient;
use sqlx::Pool;
use sqlx::Postgres;
use std::env;
use std::sync::Arc;

pub async fn setup_db(clean_db: bool) -> Result<Pool<Postgres>, Error> {
    // set the postgres database url
    unsafe {
        env::set_var(
            "DATABASE_URL",
            "postgresql://postgres:admin@localhost:5432/monitor?",
        );
        env::set_var("MAX_CONNECTIONS", "10");
    }

    // set the max connections for the postgres pool
    let pool = create_db_pool(None).await.expect("error");

    // run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    if clean_db {
        sqlx::raw_sql(
            r#"
            DELETE 
            FROM scouter.drift;

            DELETE 
            FROM scouter.drift_profile;
            "#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
    }

    Ok(pool)
}

#[allow(dead_code)]
pub async fn setup_api(clean_db: bool) -> Result<Router, Error> {
    // set the postgres database url
    let pool = setup_db(clean_db).await.unwrap();

    let db_client = PostgresClient::new(pool).unwrap();
    let router = create_router(Arc::new(AppState { db: db_client }));

    Ok(router)
}

#[allow(dead_code)]
pub async fn teardown() -> Result<(), Error> {
    // clear the database

    let pool = setup_db(true).await.unwrap();

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift;

            DELETE 
            FROM scouter.drift_profile;

            DELETE
            FROM scouter.drift_alerts;
            "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Ok(())
}
