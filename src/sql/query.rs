use chrono::NaiveDateTime;

//constants

const INSERT_DRIFT_RECORD: &str = include_str!("scripts/insert_drift_record.sql");
const GET_FEATURES: &str = include_str!("scripts/unique_features.sql");
const GET_BINNED_FEATURE_VALUES: &str = include_str!("scripts/binned_feature_values.sql");
const GET_FEATURE_VALUES: &str = include_str!("scripts/feature_values.sql");
const INSERT_DRIFT_PROFILE: &str = include_str!("scripts/insert_drift_profile.sql");
const INSERT_DRIFT_ALERT: &str = include_str!("scripts/insert_drift_alert.sql");
const GET_DRIFT_TASK: &str = include_str!("scripts/poll_for_drift_task.sql");
const GET_DRIFT_ALERTS: &str = include_str!("scripts/get_drift_alerts.sql");
const GET_DRIFT_PROFILE: &str = include_str!("scripts/get_drift_profile.sql");
const UPDATE_DRIFT_PROFILE_RUN_DATES: &str =
    include_str!("scripts/update_drift_profile_run_dates.sql");
const UPDATE_DRIFT_PROFILE_STATUS: &str = include_str!("scripts/update_drift_profile_status.sql");
const UPDATE_DRIFT_PROFILE: &str = include_str!("scripts/update_drift_profile.sql");

pub struct UpdateDriftProfileRunDatesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub next_run: NaiveDateTime,
}

pub struct UpdateDriftProfileStatusParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub active: bool,
}

pub struct GetDriftProfileParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
}

pub struct InsertParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub feature: String,
    pub value: String,
    pub created_at: NaiveDateTime,
}

pub struct GetFeaturesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
}

pub struct GetFeatureValuesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub feature: String,
    pub limit_timestamp: String,
}

pub struct InsertDriftProfileParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub profile: String,
    pub scouter_version: String,
    pub active: bool,
    pub schedule: String,
    pub next_run: NaiveDateTime,
    pub previous_run: NaiveDateTime,
}

pub struct UpdateDriftProfileParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub profile: String,
}

pub struct InsertDriftAlertParams {
    pub name: String,
    pub repository: String,
    pub version: String,
    pub alert: String,
}

pub struct GetDriftAlertsParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
}

pub struct GetBinnedFeatureValuesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub feature: String,
    pub version: String,
    pub time_window: String,
    pub bin: String,
}

#[allow(dead_code)]
pub enum Queries {
    GetFeatures,
    InsertDriftRecord,
    InsertDriftProfile,
    InsertDriftAlert,
    GetDriftAlerts,
    GetBinnedFeatureValues,
    GetFeatureValues,
    GetDriftTask,
    GetDriftProfile,
    UpdateDriftProfileRunDates,
    UpdateDriftProfileStatus,
    UpdateDriftProfile,
}

impl Queries {
    pub fn get_query(&self) -> SqlQuery {
        match self {
            // load sql file from scripts/insert.sql
            Queries::GetFeatures => SqlQuery::new(GET_FEATURES),
            Queries::InsertDriftRecord => SqlQuery::new(INSERT_DRIFT_RECORD),
            Queries::GetBinnedFeatureValues => SqlQuery::new(GET_BINNED_FEATURE_VALUES),
            Queries::GetFeatureValues => SqlQuery::new(GET_FEATURE_VALUES),
            Queries::InsertDriftProfile => SqlQuery::new(INSERT_DRIFT_PROFILE),
            Queries::InsertDriftAlert => SqlQuery::new(INSERT_DRIFT_ALERT),
            Queries::GetDriftAlerts => SqlQuery::new(GET_DRIFT_ALERTS),
            Queries::GetDriftTask => SqlQuery::new(GET_DRIFT_TASK),
            Queries::UpdateDriftProfileRunDates => SqlQuery::new(UPDATE_DRIFT_PROFILE_RUN_DATES),
            Queries::UpdateDriftProfileStatus => SqlQuery::new(UPDATE_DRIFT_PROFILE_STATUS),
            Queries::UpdateDriftProfile => SqlQuery::new(UPDATE_DRIFT_PROFILE),
            Queries::GetDriftProfile => SqlQuery::new(GET_DRIFT_PROFILE),
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
