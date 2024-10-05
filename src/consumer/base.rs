use crate::sql::postgres::PostgresClient;
use crate::sql::schema::{DriftRecord, ObservabilityRecord};
use anyhow::*;
use scouter::core::drift::base::{RecordType, ServerRecord, ServerRecords};
use std::result::Result::Ok;
use tracing::error;

pub trait ToDriftRecords {
    fn to_spc_drift_records(&self) -> Result<Vec<DriftRecord>>;
    fn to_observability_drift_records(&self) -> Result<Vec<ObservabilityRecord>>;
}
impl ToDriftRecords for ServerRecords {
    fn to_spc_drift_records(&self) -> Result<Vec<DriftRecord>> {
        match self.record_type {
            RecordType::SPC => {
                let mut records = Vec::new();
                for record in self.records.iter() {
                    match record {
                        ServerRecord::SPC {
                            record: inner_record,
                        } => {
                            records.push(DriftRecord {
                                repository: inner_record.repository.clone(),
                                name: inner_record.name.clone(),
                                version: inner_record.version.clone(),
                                created_at: inner_record.created_at,
                                feature: inner_record.feature.clone(),
                                value: inner_record.value,
                            });
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

    fn to_observability_drift_records(&self) -> Result<Vec<ObservabilityRecord>> {
        match self.record_type {
            RecordType::SPC => todo!(),
            RecordType::OBSERVABILITY => {
                let mut records = Vec::new();
                for record in self.records.iter() {
                    match record {
                        ServerRecord::OBSERVABILITY {
                            record: inner_record,
                        } => {
                            records.push(ObservabilityRecord {
                                repository: inner_record.repository.clone(),
                                name: inner_record.name.clone(),
                                version: inner_record.version.clone(),
                                request_count: inner_record.request_count.clone(),
                                error_count: inner_record.error_count.clone(),
                                route_metrics: inner_record.route_metrics.clone(),
                            });
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
                            let _ = client.insert_drift_record(record).await.map_err(|e| {
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
