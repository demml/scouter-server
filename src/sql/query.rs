use std::collections::BTreeMap;

pub trait ToMap {
    fn to_map(&self) -> BTreeMap<String, String>;
}

pub struct InsertParams {
    pub table: String,
    pub service_name: String,
    pub feature: String,
    pub value: String,
    pub version: String,
}

impl ToMap for InsertParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("service_name".to_string(), self.service_name.clone());
        params.insert("feature".to_string(), self.feature.clone());
        params.insert("value".to_string(), self.value.clone());
        params.insert("version".to_string(), self.version.clone());

        params
    }
}

pub struct GetFeaturesParams {
    pub table: String,
    pub service_name: String,
    pub version: String,
}

impl ToMap for GetFeaturesParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("service_name".to_string(), self.service_name.clone());
        params.insert("version".to_string(), self.version.clone());

        params
    }
}

pub struct GetFeatureValuesParams {
    pub table: String,
    pub service_name: String,
    pub feature: String,
    pub version: String,
    pub time_window: String,
    pub bin: String,
}

impl ToMap for GetFeatureValuesParams {
    fn to_map(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("table".to_string(), self.table.clone());
        params.insert("service_name".to_string(), self.service_name.clone());
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
    GetFeatureValues,
}

impl Queries {
    pub fn get_query(&self) -> SqlQuery {
        match self {

            Queries::GetFeatures => SqlQuery::new("SELECT DISTINCT feature FROM $table WHERE service_name = '$service_name' AND version = '$version';"),
            Queries::InsertDriftRecord => SqlQuery::new("INSERT INTO $table (service_name, feature, value, version) VALUES ('$service_name', '$feature', '$value', '$version');"),
            Queries::GetFeatureValues => SqlQuery::new("with subquery as (
                SELECT 
                date_bin('$bin minutes', created_at, TIMESTAMP '1970-01-01') as created_at,
                service_name,
                feature,
                version,
                value
                from $table
                WHERE 
                    created_at > timezone('utc', now()) - interval '$time_window' minute
                    AND version = '$version'
                    AND service_name = '$service_name'
                    AND feature = '$feature'
            )

            SELECT
            created_at,
            service_name,
            feature,
            version,
            avg(value) as value
            FROM subquery
            GROUP BY 
                created_at,
                service_name,
                feature,
                version
            ORDER BY
                created_at DESC;
            "),
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

    #[test]
    pub fn test_insert_query() {
        let query = Queries::InsertDriftRecord.get_query();

        let params = InsertParams {
            table: "features".to_string(),
            service_name: "test".to_string(),
            feature: "test".to_string(),
            value: "test".to_string(),
            version: "test".to_string(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(formatted_sql, "INSERT INTO features (service_name, feature, value, version) VALUES ('test', 'test', 'test', 'test');");
    }

    #[test]
    fn test_get_features_query() {
        let query = Queries::GetFeatures.get_query();

        let params = GetFeaturesParams {
            table: "schema.table".to_string(),
            service_name: "test".to_string(),
            version: "test".to_string(),
        };

        let formatted_sql = query.format(&params);

        assert_eq!(
            formatted_sql,
            "SELECT DISTINCT feature FROM schema.table WHERE service_name = 'test' AND version = 'test';"
        );
    }

    #[test]
    fn test_get_features_values_query() {
        let query = Queries::GetFeatureValues.get_query();

        let params = GetFeatureValuesParams {
            table: "schema.table".to_string(),
            service_name: "test".to_string(),
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
                service_name,
                feature,
                version,
                value
                from schema.table
                WHERE 
                    created_at > timezone('utc', now()) - interval '10' minute
                    AND version = 'test'
                    AND service_name = 'test'
                    AND feature = 'test'
            )

            SELECT
            created_at,
            service_name,
            feature,
            version,
            avg(value) as value
            FROM subquery
            GROUP BY 
                created_at,
                service_name,
                feature,
                version
            ORDER BY
                created_at DESC;
            "
        );
    }
}
