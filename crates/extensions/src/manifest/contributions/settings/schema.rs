use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{ManifestError, SettingsError};

const LARGEST_VALUES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value", into = "Value")]
pub struct SettingsSchema(Value);

impl SettingsSchema {
    pub(crate) fn check(&self, values: &Value) -> Result<(), SettingsError> {
        if values.to_string().len() > LARGEST_VALUES {
            return Err(SettingsError::TooLarge(LARGEST_VALUES));
        }

        let values = values.as_object().ok_or(SettingsError::NotObject)?;

        self.check_required(values)?;

        values
            .iter()
            .try_for_each(|(name, value)| self.check_property(name, value))
    }

    fn check_required(&self, values: &Map<String, Value>) -> Result<(), SettingsError> {
        let required = self
            .0
            .get("required")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);

        let missing = required
            .iter()
            .filter_map(Value::as_str)
            .find(|name| !values.contains_key(*name));

        match missing {
            Some(name) => Err(SettingsError::Required(name.to_owned())),
            None => Ok(()),
        }
    }

    fn check_property(&self, name: &str, value: &Value) -> Result<(), SettingsError> {
        let property = self
            .0
            .get("properties")
            .and_then(|properties| properties.get(name))
            .ok_or_else(|| SettingsError::Unknown(name.to_owned()))?;

        if let Some(kind) = property.get("type").and_then(Value::as_str)
            && !Self::has_type(value, kind)
        {
            return Err(SettingsError::WrongType {
                name: name.to_owned(),
                kind: kind.to_owned(),
            });
        }

        let allowed = property.get("enum").and_then(Value::as_array);

        if allowed.is_some_and(|allowed| !allowed.contains(value)) {
            return Err(SettingsError::NotListed(name.to_owned()));
        }

        Ok(())
    }

    fn has_type(value: &Value, kind: &str) -> bool {
        match kind {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.is_i64() || value.is_u64(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => false,
        }
    }
}

impl TryFrom<Value> for SettingsSchema {
    type Error = ManifestError;

    fn try_from(schema: Value) -> Result<Self, ManifestError> {
        let is_object = schema.get("type").and_then(Value::as_str) == Some("object");
        let has_properties = schema.get("properties").is_some_and(Value::is_object);

        if !is_object || !has_properties {
            return Err(ManifestError::SettingsSchema);
        }

        Ok(Self(schema))
    }
}

impl From<SettingsSchema> for Value {
    fn from(schema: SettingsSchema) -> Self {
        schema.0
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SettingsSchema;

    fn schema() -> SettingsSchema {
        SettingsSchema::try_from(json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "every": { "type": "integer" },
                "unit": { "type": "string", "enum": ["s", "m"] }
            },
            "required": ["url"]
        }))
        .unwrap()
    }

    #[test]
    fn accepts_values_that_match_the_schema() {
        let checked = schema().check(&json!({ "url": "https://x", "every": 5, "unit": "m" }));

        assert_eq!(checked, Ok(()));
    }

    #[test]
    fn refuses_unknown_wrongly_typed_missing_and_unlisted_values() {
        let schema = schema();

        for values in [
            json!({ "url": "https://x", "color": "red" }),
            json!({ "url": 5 }),
            json!({ "url": "https://x", "every": 1.5 }),
            json!({ "every": 5 }),
            json!({ "url": "https://x", "unit": "h" }),
            json!(["url"]),
        ] {
            assert!(schema.check(&values).is_err(), "{values}");
        }
    }
}
