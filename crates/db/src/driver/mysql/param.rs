use mysql_async::Value as MysqlValue;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Param(MysqlValue);

impl From<&Value> for Param {
    fn from(value: &Value) -> Self {
        let converted = match value {
            Value::Null => MysqlValue::NULL,
            Value::Bool(flag) => MysqlValue::Int(i64::from(*flag)),
            Value::Number(number) => number
                .as_i64()
                .map(MysqlValue::Int)
                .or_else(|| number.as_u64().map(MysqlValue::UInt))
                .or_else(|| number.as_f64().map(MysqlValue::Double))
                .unwrap_or(MysqlValue::NULL),
            Value::String(text) => MysqlValue::Bytes(text.clone().into_bytes()),
            other => MysqlValue::Bytes(other.to_string().into_bytes()),
        };

        Self(converted)
    }
}

impl From<Param> for MysqlValue {
    fn from(param: Param) -> Self {
        param.0
    }
}

#[cfg(test)]
mod tests {
    use mysql_async::Value as MysqlValue;
    use serde_json::{Value, json};

    use super::Param;

    fn param(value: &Value) -> MysqlValue {
        Param::from(value).into()
    }

    #[test]
    fn json_values_become_the_values_mysql_binds() {
        assert_eq!(param(&json!(null)), MysqlValue::NULL);
        assert_eq!(param(&json!(true)), MysqlValue::Int(1));
        assert_eq!(param(&json!(u64::MAX)), MysqlValue::UInt(u64::MAX));
        assert_eq!(param(&json!(2.5)), MysqlValue::Double(2.5));
        assert_eq!(param(&json!("ann")), MysqlValue::Bytes(b"ann".to_vec()));
        assert_eq!(param(&json!([1])), MysqlValue::Bytes(b"[1]".to_vec()));
    }
}
