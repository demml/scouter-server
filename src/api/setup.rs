use anyhow::Context;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tracing::{error, info};

use std::io;

use tracing_subscriber;
use tracing_subscriber::fmt::time::UtcTime;

const DEFAULT_TIME_PATTERN: &str =
    "[year]-[month]-[day]T[hour repr:24]:[minute]:[second]::[subsecond digits:4]";

pub async fn setup_logging() -> Result<(), anyhow::Error> {
    let time_format = time::format_description::parse(DEFAULT_TIME_PATTERN).unwrap();

    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .flatten_event(true)
        .with_thread_ids(true)
        .with_timer(UtcTime::new(time_format))
        .with_writer(io::stdout)
        .init();

    Ok(())
}

/// Setup the application with the given database pool.

pub async fn setup_db(database_url: Option<String>) -> Result<Pool<Postgres>, anyhow::Error> {
    let database_url = match database_url {
        Some(url) => url,
        None => std::env::var("DATABASE_URL").with_context(|| "DATABASE_URL must be set")?,
    };

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

    Ok(pool)
}
