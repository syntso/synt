use rusqlite::ToSql;
use rusqlite::types::{ToSqlOutput, Value as SqliteValue};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Param(SqliteValue);

impl From<&Value> for Param {
    fn from(value: &Value) -> Self {
        let converted = match value {
            Value::Null => SqliteValue::Null,
            Value::Bool(flag) => SqliteValue::Integer(i64::from(*flag)),
            Value::Number(number) => number
                .as_i64()
                .map(SqliteValue::Integer)
                .or_else(|| number.as_f64().map(SqliteValue::Real))
                .unwrap_or(SqliteValue::Null),
            Value::String(text) => SqliteValue::Text(text.clone()),
            other => SqliteValue::Text(other.to_string()),
        };

        Self(converted)
    }
}

impl ToSql for Param {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::types::Value as SqliteValue;
    use serde_json::json;

    use super::Param;

    #[test]
    fn json_params_map_to_storage_classes() {
        assert_eq!(Param::from(&json!(true)), Param(SqliteValue::Integer(1)));
        assert_eq!(Param::from(&json!(2.5)), Param(SqliteValue::Real(2.5)));
        assert_eq!(
            Param::from(&json!("x")),
            Param(SqliteValue::Text("x".to_owned()))
        );
        assert_eq!(
            Param::from(&json!([1])),
            Param(SqliteValue::Text("[1]".to_owned()))
        );
    }
}
