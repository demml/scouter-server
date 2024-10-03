use crate::sql::postgres::PostgresClient;
use crate::sql::schema::DriftRecord;
use anyhow::*;
use scouter::core::drift::base::{RecordType, ServerRecord, ServerRecords};
use std::result::Result::Ok;
use tracing::error;

pub trait ToDriftRecords {
    fn to_spc_drift_records(&self) -> Result<Vec<DriftRecord>>;
}
impl ToDriftRecords for ServerRecords {
    fn to_spc_drift_records(&self) -> Result<Vec<DriftRecord>> {
        match self.record_type {
            RecordType::DRIFT => {
                let mut records = Vec::new();
                for record in self.records.iter() {
                    let ServerRecord::DRIFT {
                        record: inner_record,
                    } = record;
                    {
                        records.push(DriftRecord {
                            repository: inner_record.repository.clone(),
                            name: inner_record.name.clone(),
                            version: inner_record.version.clone(),
                            created_at: inner_record.created_at,
                            feature: inner_record.feature.clone(),
                            value: inner_record.value,
                        });
                    }
                }
                Ok(records)
            }
            RecordType::OBSERVABILITY => todo!(),
        }
    }
}

pub enum MessageHandler {
    Postgres(PostgresClient),
}

impl MessageHandler {
    pub async fn insert_server_records(&self, records: &ServerRecords) -> Result<()> {
        match self {
            Self::Postgres(client) => {
                match records.record_type {
                    RecordType::DRIFT => {
                        let records = records.to_spc_drift_records()?;
                        for record in records.iter() {
                            let _ = client.insert_drift_record(record).await.map_err(|e| {
                                error!("Failed to insert drift record: {:?}", e);
                            });
                        }
                    }
                    RecordType::OBSERVABILITY => todo!(),
                };
            }
        }

        Ok(())
    }
}
