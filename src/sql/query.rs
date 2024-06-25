use crate::sql::schema::MonitorProfile;
use chrono::NaiveDateTime;
use sqlx::types::Json;
use std::collections::BTreeMap;

//constants

const INSERT_DRIFT_RECORD: &'static str = include_str!("scripts/insert_drift_record.sql");
const GET_FEATURES: &'static str = include_str!("scripts/unique_features.sql");
const GET_BINNED_FEATURE_VALUES: &'static str = include_str!("scripts/binned_feature_values.sql");
const GET_FEATURE_VALUES: &'static str = include_str!("scripts/feature_values.sql");
const INSERT_DRIFT_PROFILE: &'static str = include_str!("scripts/insert_drift_profile.sql");

pub trait ToMap {
    fn to_map(&self) -> BTreeMap<String, String>;
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

pub struct InsertMonitorProfileParams {
    pub table: String,
    pub name: String,
    pub repository: String,
    pub version: String,
    pub profile: String,
    pub cron: String,
    pub next_run: NaiveDateTime,
}

impl ToMap for InsertMonitorProfileParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("name".to_string(), self.name.clone());
        params.insert("repository".to_string(), self.repository.clone());
        params.insert("version".to_string(), self.version.clone());
        params.insert("profile".to_string(), self.profile.clone());
        params.insert("cron".to_string(), self.cron.clone());
        params.insert("next_run".to_string(), self.next_run.to_string());

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

pub enum Queries {
    GetFeatures,
    InsertDriftRecord,
    InsertMonitorProfile,
    GetBinnedFeatureValues,
    GetFeatureValues,
}

impl Queries {
    pub fn get_query(&self) -> SqlQuery {
        match self {
            // load sql file from scripts/insert.sql
            Queries::GetFeatures => SqlQuery::new(GET_FEATURES),
            Queries::InsertDriftRecord => SqlQuery::new(INSERT_DRIFT_RECORD),
            Queries::GetBinnedFeatureValues => SqlQuery::new(GET_BINNED_FEATURE_VALUES),
            Queries::GetFeatureValues => SqlQuery::new(GET_FEATURE_VALUES),
            Queries::InsertMonitorProfile => SqlQuery::new(&INSERT_DRIFT_PROFILE),
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
            format!("INSERT INTO features (created_at, name, repository, version, feature, value) \nVALUES ('{}', 'test', 'test', 'test', 'test', 'test')\nON CONFLICT DO NOTHING;", params.created_at.to_string())
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
value,
version
FROM schema.table
WHERE
    created_at > timezone('utc', now()) - interval '10' minute
    AND version = 'test'
    AND name = 'test'
    AND repository = 'test'
    AND feature = 'test';"
        );
    }
}
