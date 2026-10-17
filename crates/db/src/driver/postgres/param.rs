use std::error::Error;

use bytes::BytesMut;
use postgres_protocol::types::text_to_sql;
use serde_json::Value;
use tokio_postgres::types::{Format, IsNull, ToSql, Type, to_sql_checked};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Param(Option<String>);

impl From<&Value> for Param {
    fn from(value: &Value) -> Self {
        let text = match value {
            Value::Null => None,
            Value::String(text) => Some(text.clone()),
            other => Some(other.to_string()),
        };

        Self(text)
    }
}

impl ToSql for Param {
    fn to_sql(
        &self,
        _type: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let Some(text) = &self.0 else {
            return Ok(IsNull::Yes);
        };

        text_to_sql(text, out);
        Ok(IsNull::No)
    }

    fn accepts(_type: &Type) -> bool {
        true
    }

    fn encode_format(&self, _type: &Type) -> Format {
        Format::Text
    }

    to_sql_checked!();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Param;

    #[test]
    fn json_values_become_the_text_postgres_parses() {
        assert_eq!(Param::from(&json!("ann")), Param(Some("ann".to_owned())));
        assert_eq!(Param::from(&json!(42)), Param(Some("42".to_owned())));
        assert_eq!(Param::from(&json!(true)), Param(Some("true".to_owned())));
        assert_eq!(
            Param::from(&json!({"a": 1})),
            Param(Some(r#"{"a":1}"#.to_owned()))
        );
        assert_eq!(Param::from(&json!(null)), Param(None));
    }
}
