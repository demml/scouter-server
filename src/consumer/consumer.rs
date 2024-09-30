use crate::sql::postgres::PostgresClient;
use anyhow::*;
use scouter::core::spc::types::SpcDriftServerRecord;
use std::result::Result::Ok;
use tracing::error;

pub enum MessageHandler {
    Postgres(PostgresClient),
}

impl MessageHandler {
    pub async fn insert_drift_record(&self, records: &SpcDriftServerRecord) -> Result<()> {
        match self {
            Self::Postgres(client) => {
                let result = client.insert_drift_record(records).await;
                match result {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Failed to insert drift record: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }
}
