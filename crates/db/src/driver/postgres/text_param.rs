use std::error::Error;

use bytes::BytesMut;
use postgres_protocol::types::text_to_sql;
use serde_json::Value;
use tokio_postgres::types::{Format, IsNull, ToSql, Type, to_sql_checked};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TextParam(Option<String>);

impl TextParam {
    pub(super) fn from_json(value: &Value) -> Self {
        let text = match value {
            Value::Null => None,
            Value::String(text) => Some(text.clone()),
            other => Some(other.to_string()),
        };

        Self(text)
    }
}

impl ToSql for TextParam {
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

    use super::TextParam;

    #[test]
    fn json_values_become_the_text_postgres_parses() {
        assert_eq!(
            TextParam::from_json(&json!("ann")),
            TextParam(Some("ann".to_owned()))
        );
        assert_eq!(
            TextParam::from_json(&json!(42)),
            TextParam(Some("42".to_owned()))
        );
        assert_eq!(
            TextParam::from_json(&json!(true)),
            TextParam(Some("true".to_owned()))
        );
        assert_eq!(
            TextParam::from_json(&json!({"a": 1})),
            TextParam(Some(r#"{"a":1}"#.to_owned()))
        );
        assert_eq!(TextParam::from_json(&json!(null)), TextParam(None));
    }
}
