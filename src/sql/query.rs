use chrono::NaiveDateTime;
use std::collections::BTreeMap;

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
const GET_FEATURE_DISTRIBUTION: &str = include_str!("scripts/get_feature_distribution.sql");
const UPDATE_DRIFT_PROFILE_RUN_DATES: &str =
    include_str!("scripts/update_drift_profile_run_dates.sql");
const UPDATE_DRIFT_PROFILE_STATUS: &str = include_str!("scripts/update_drift_profile_status.sql");
const UPDATE_DRIFT_PROFILE: &str = include_str!("scripts/update_drift_profile.sql");

// table names
pub const DRIFT_TABLE: &str = "scouter.drift";
pub const DRIFT_PROFILE_TABLE: &str = "scouter.drift_profile";
pub const DRIFT_ALERT_TABLE: &str = "scouter.drift_alerts";

pub trait ToMap {
    fn to_map(&self) -> BTreeMap<String, String>;
}

pub struct UpdateDriftProfileRunDatesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub next_run: NaiveDateTime,
}

impl ToMap for UpdateDriftProfileRunDatesParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("next_run".to_string(), self.next_run.to_string());
        params
    }
}

pub struct UpdateDriftProfileStatusParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub active: bool,
}

impl ToMap for UpdateDriftProfileStatusParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("active".to_string(), self.active.to_string());
        params
    }
}

pub struct GetDriftProfileParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
}

impl ToMap for GetDriftProfileParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params
    }
}

pub struct GetDriftProfileTaskParams {
    pub table: String,
}

impl ToMap for GetDriftProfileTaskParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params
    }
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

impl ToMap for InsertParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("feature".to_string(), self.feature.clone());
        params.insert("value".to_string(), self.value.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("created_at".to_string(), self.created_at.to_string());

        params
    }
}

pub struct GetFeaturesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
}

impl ToMap for GetFeaturesParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());

        params
    }
}

pub struct GetFeatureValuesParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub feature: String,
    pub limit_timestamp: String,
}

impl ToMap for GetFeatureValuesParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("feature".to_string(), self.feature.clone());
        params.insert("limit_timestamp".to_string(), self.limit_timestamp.clone());

        params
    }
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

impl ToMap for InsertDriftProfileParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("profile".to_string(), self.profile.clone());
        params.insert("scouter_version".to_string(), self.scouter_version.clone());
        params.insert("active".to_string(), self.active.to_string());
        params.insert("schedule".to_string(), self.schedule.clone());
        params.insert("next_run".to_string(), self.next_run.to_string());
        params.insert("previous_run".to_string(), self.previous_run.to_string());

        params
    }
}

pub struct UpdateDriftProfileParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub profile: String,
}

impl ToMap for UpdateDriftProfileParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("profile".to_string(), self.profile.clone());

        params
    }
}

pub struct InsertDriftAlertParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub alert: String,
}

impl ToMap for InsertDriftAlertParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("alert".to_string(), self.alert.clone());
        params
    }
}

pub struct GetDriftAlertsParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
}

impl ToMap for GetDriftAlertsParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params
    }
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

impl ToMap for GetBinnedFeatureValuesParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("feature".to_string(), self.feature.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("time_window".to_string(), self.time_window.clone());
        params.insert("bin".to_string(), self.bin.clone());

        params
    }
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
    GetFeatureDistribution,
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
            Queries::GetFeatureDistribution => SqlQuery::new(GET_FEATURE_DISTRIBUTION),
        }
    }
}

pub struct SqlQuery {
    sql: String,
}

impl SqlQuery {
    fn new(sql: &str) -> Self {
        Self {
            sql: sql.to_string(),
        }
    }

    pub fn format<T>(&self, params: &T) -> String
    where
        T: ToMap,
    {
        let mut formatted_sql = self.sql.clone();
        let params = params.to_map();

        for (key, value) in params {
            formatted_sql = formatted_sql.replace(&format!("${}", key), &value);
        }

        formatted_sql
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use chrono;

    #[test]
    pub fn test_insert_query() {
        let query = Queries::InsertDriftRecord.get_query();

        let params = InsertParams {
            table: "features".to_string(),
            name: "test".to_string(),
            repository: "test".to_string(),
            feature: "test".to_string(),
            value: "test".to_string(),
            version: "test".to_string(),
            created_at: chrono::Utc::now().naive_utc(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            format!("INSERT INTO features (created_at, name, repository, version, feature, value) \nVALUES ('{}', 'test', 'test', 'test', 'test', 'test')\nON CONFLICT DO NOTHING;", params.created_at)
        );
    }

    #[test]
    fn test_get_features_query() {
        let query = Queries::GetFeatures.get_query();

        let params = GetFeaturesParams {
            table: "schema.table".to_string(),
            name: "test".to_string(),
            repository: "test".to_string(),
            version: "test".to_string(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            "SELECT
DISTINCT feature
FROM schema.table
WHERE
   name = 'test'
   AND repository = 'test'
   AND version = 'test';"
        );
    }

    #[test]
    fn test_get_drift_profile_query() {
        let query = Queries::GetDriftTask.get_query();

        let params = GetDriftProfileTaskParams {
            table: "schema.table".to_string(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            "SELECT name, repository, version, profile, previous_run, schedule\nFROM schema.table\nWHERE active\n  AND next_run < CURRENT_TIMESTAMP\nLIMIT 1 FOR UPDATE SKIP LOCKED;"
        );
    }

    #[test]
    fn test_update_drift_profile_run_dates_query() {
        let query = Queries::UpdateDriftProfileRunDates.get_query();

        let params = UpdateDriftProfileRunDatesParams {
            table: "schema.table".to_string(),
            name: "name".to_string(),
            repository: "repository".to_string(),
            version: "version".to_string(),
            next_run: Default::default(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            "UPDATE schema.table
SET previous_run = next_run,
    next_run     = '1970-01-01 00:00:00',
    updated_at   = timezone('utc', now())
WHERE name = 'name'
  and repository = 'repository'
  and version = 'version';"
        );
    }

    #[test]
    fn test_get_features_values_query() {
        let query = Queries::GetBinnedFeatureValues.get_query();

        let params = GetBinnedFeatureValuesParams {
            table: "schema.table".to_string(),
            name: "test".to_string(),
            repository: "test".to_string(),
            feature: "test".to_string(),
            version: "test".to_string(),
            time_window: "10".to_string(),
            bin: "1".to_string(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            "with subquery as (
    SELECT
    date_bin('1 minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
    name,
    repository,
    feature,
    version,
    value
    from schema.table
    WHERE 
        created_at > timezone('utc', now()) - interval '10' minute
        AND version = 'test'
        AND name = 'test'
        AND repository = 'test'
        AND feature = 'test'
)

SELECT
created_at,
name,
repository,
feature,
version,
avg(value) as value
FROM subquery
GROUP BY 
    created_at,
    name,
    repository,
    feature,
    version
ORDER BY
    created_at DESC;"
        );
    }

    #[test]
    fn test_get_feature_values_query() {
        let query = Queries::GetFeatureValues.get_query();

        let params = GetFeatureValuesParams {
            table: "schema.table".to_string(),
            name: "test".to_string(),
            repository: "test".to_string(),
            feature: "test".to_string(),
            version: "test".to_string(),
            limit_timestamp: "2024-01-01 00:00:00".to_string(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            "SELECT
created_at,
feature,
value
FROM schema.table
WHERE
    created_at > '2024-01-01 00:00:00'
    AND version = 'test'
    AND name = 'test'
    AND repository = 'test'
    AND feature = 'test';"
        );
    }
}
