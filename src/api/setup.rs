use crate::sql::postgres::PostgresClient;
use anyhow::Context;
use std::io;

use tracing_subscriber;
use tracing_subscriber::fmt::time::UtcTime;

const DEFAULT_TIME_PATTERN: &str =
    "[year]-[month]-[day]T[hour repr:24]:[minute]:[second]::[subsecond digits:4]";

pub async fn setup() -> Result<PostgresClient, anyhow::Error> {
    let time_format = time::format_description::parse(DEFAULT_TIME_PATTERN).unwrap();

    tracing_subscriber::fmt()
        .json()
        .with_target(false)
        .flatten_event(true)
        .with_thread_ids(true)
        .with_timer(UtcTime::new(time_format))
        .with_writer(io::stdout)
        .init();

    let db_client = PostgresClient::new(None)
        .await
        .with_context(|| "Failed to create Postgres client")?;

    Ok(db_client)
}
