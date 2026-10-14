use mysql_async::{Params, Value as MysqlValue};
use serde_json::Value;

pub(super) fn params(values: &[Value]) -> Params {
    Params::Positional(values.iter().map(value).collect())
}

fn value(value: &Value) -> MysqlValue {
    match value {
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
    }
}
