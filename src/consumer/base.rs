use crate::sql::postgres::PostgresClient;
use anyhow::*;
use scouter::core::drift::base::{RecordType, ServerRecord, ServerRecords};
use scouter::core::drift::spc::types::SpcServerRecord;
use scouter::core::observe::observer::ObservabilityMetrics;
use std::result::Result::Ok;
use tracing::error;

pub trait ToDriftRecords {
    fn to_spc_drift_records(&self) -> Result<Vec<SpcServerRecord>>;
    fn to_observability_drift_records(&self) -> Result<Vec<ObservabilityMetrics>>;
}
impl ToDriftRecords for ServerRecords {
    fn to_spc_drift_records(&self) -> Result<Vec<SpcServerRecord>> {
        match self.record_type {
            RecordType::SPC => {
                let mut records = Vec::new();
                for record in self.records.iter() {
                    match record {
                        ServerRecord::SPC {
                            record: inner_record,
                        } => {
                            records.push(inner_record.clone());
                        }
                        _ => {
                            error!("Unexpected record type");
                        }
                    }
                }
                Ok(records)
            }
            RecordType::OBSERVABILITY => todo!(),
            RecordType::PSI => todo!(),
        }
    }

    fn to_observability_drift_records(&self) -> Result<Vec<ObservabilityMetrics>> {
        match self.record_type {
            RecordType::SPC => todo!(),
            RecordType::OBSERVABILITY => {
                let mut records = Vec::new();
                for record in self.records.iter() {
                    match record {
                        ServerRecord::OBSERVABILITY {
                            record: inner_record,
                        } => {
                            records.push(inner_record.clone());
                        }
                        _ => {
                            error!("Unexpected record type");
                        }
                    }
                }
                Ok(records)
            }
            RecordType::PSI => todo!(),
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
                    RecordType::SPC => {
                        let records = records.to_spc_drift_records()?;
                        for record in records.iter() {
                            let _ = client.insert_spc_drift_record(record).await.map_err(|e| {
                                error!("Failed to insert drift record: {:?}", e);
                            });
                        }
                    }
                    RecordType::OBSERVABILITY => {
                        let records = records.to_observability_drift_records()?;
                        for record in records.iter() {
                            let _ = client
                                .insert_observability_record(record)
                                .await
                                .map_err(|e| {
                                    error!("Failed to insert observability record: {:?}", e);
                                });
                        }
                    }
                    RecordType::PSI => todo!(),
                };
            }
        }

        Ok(())
    }
}
