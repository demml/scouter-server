//constants

const INSERT_DRIFT_RECORD: &str = include_str!("scripts/insert_drift_record.sql");
const GET_FEATURES: &str = include_str!("scripts/unique_features.sql");
const GET_BINNED_FEATURE_VALUES: &str = include_str!("scripts/binned_feature_values.sql");
const GET_BINNED_ALERT_VALUES: &str = include_str!("scripts/get_alert_metrics.sql");
const GET_FEATURE_VALUES: &str = include_str!("scripts/feature_values.sql");
const GET_BINNED_OBSERVABILITY_METRICS: &str =
    include_str!("scripts/binned_observability_metrics.sql");
const INSERT_DRIFT_PROFILE: &str = include_str!("scripts/insert_drift_profile.sql");
const INSERT_DRIFT_ALERT: &str = include_str!("scripts/insert_drift_alert.sql");
const INSERT_OBSERVABILITY_RECORD: &str = include_str!("scripts/insert_observability_record.sql");
const GET_DRIFT_TASK: &str = include_str!("scripts/poll_for_drift_task.sql");
const GET_DRIFT_ALERTS: &str = include_str!("scripts/get_drift_alerts.sql");
const GET_DRIFT_PROFILE: &str = include_str!("scripts/get_drift_profile.sql");
const GET_FEATURE_DISTRIBUTION: &str = include_str!("scripts/get_feature_distribution.sql");
const UPDATE_DRIFT_PROFILE_RUN_DATES: &str =
    include_str!("scripts/update_drift_profile_run_dates.sql");
const UPDATE_DRIFT_PROFILE_STATUS: &str = include_str!("scripts/update_drift_profile_status.sql");
const UPDATE_DRIFT_PROFILE: &str = include_str!("scripts/update_drift_profile.sql");
const UPDATE_DRIFT_ALERT: &str = include_str!("scripts/update_drift_alert.sql");

#[allow(dead_code)]
pub enum Queries {
    GetFeatures,
    InsertDriftRecord,
    InsertDriftProfile,
    InsertDriftAlert,
    InsertObservabilityRecord,
    GetDriftAlerts,
    GetBinnedFeatureValues,
    GetBinnedObservabilityMetrics,
    GetFeatureValues,
    GetDriftTask,
    GetDriftProfile,
    GetFeatureDistribution,
    UpdateDriftProfileRunDates,
    UpdateDriftProfileStatus,
    UpdateDriftProfile,
    UpdateDriftAlert,
}

impl Queries {
    pub fn get_query(&self) -> SqlQuery {
        match self {
            // load sql file from scripts/insert.sql
            Queries::GetFeatures => SqlQuery::new(GET_FEATURES),
            Queries::InsertDriftRecord => SqlQuery::new(INSERT_DRIFT_RECORD),
            Queries::GetBinnedFeatureValues => SqlQuery::new(GET_BINNED_FEATURE_VALUES),
            Queries::GetBinnedObservabilityMetrics => {
                SqlQuery::new(GET_BINNED_OBSERVABILITY_METRICS)
            }
            Queries::GetFeatureValues => SqlQuery::new(GET_FEATURE_VALUES),
            Queries::InsertDriftProfile => SqlQuery::new(INSERT_DRIFT_PROFILE),
            Queries::InsertDriftAlert => SqlQuery::new(INSERT_DRIFT_ALERT),
            Queries::InsertObservabilityRecord => SqlQuery::new(INSERT_OBSERVABILITY_RECORD),
            Queries::GetDriftAlerts => SqlQuery::new(GET_DRIFT_ALERTS),
            Queries::GetDriftTask => SqlQuery::new(GET_DRIFT_TASK),
            Queries::UpdateDriftProfileRunDates => SqlQuery::new(UPDATE_DRIFT_PROFILE_RUN_DATES),
            Queries::UpdateDriftProfileStatus => SqlQuery::new(UPDATE_DRIFT_PROFILE_STATUS),
            Queries::UpdateDriftProfile => SqlQuery::new(UPDATE_DRIFT_PROFILE),
            Queries::UpdateDriftAlert => SqlQuery::new(UPDATE_DRIFT_ALERT),
            Queries::GetDriftProfile => SqlQuery::new(GET_DRIFT_PROFILE),
            Queries::GetFeatureDistribution => SqlQuery::new(GET_FEATURE_DISTRIBUTION),
        }
    }
}

pub struct SqlQuery {
    pub sql: String,
}

impl SqlQuery {
    fn new(sql: &str) -> Self {
        Self {
            sql: sql.to_string(),
        }
    }
}
