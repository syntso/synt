use serde_json::{Map, Value};

const LARGEST_VALUES: usize = 64 * 1024;

pub(crate) struct SettingsCheck<'a> {
    schema: &'a Value,
}

impl<'a> SettingsCheck<'a> {
    pub(crate) fn new(schema: &'a Value) -> Self {
        Self { schema }
    }

    pub(crate) fn check(&self, values: &Value) -> Result<(), String> {
        if values.to_string().len() > LARGEST_VALUES {
            return Err(format!("settings hold at most {LARGEST_VALUES} bytes"));
        }

        let values = values.as_object().ok_or("settings are an object")?;
        let properties = self
            .schema
            .get("properties")
            .and_then(Value::as_object)
            .ok_or("the settings schema has no properties")?;

        self.check_required(values)?;

        values
            .iter()
            .try_for_each(|(name, value)| check_property(properties, name, value))
    }

    fn check_required(&self, values: &Map<String, Value>) -> Result<(), String> {
        let required = self
            .schema
            .get("required")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);

        let missing = required
            .iter()
            .filter_map(Value::as_str)
            .find(|name| !values.contains_key(*name));

        match missing {
            Some(name) => Err(format!("the setting {name} is required")),
            None => Ok(()),
        }
    }
}

fn check_property(
    properties: &Map<String, Value>,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    let property = properties
        .get(name)
        .ok_or_else(|| format!("{name} is not a setting of this extension"))?;

    if let Some(kind) = property.get("type").and_then(Value::as_str)
        && !has_type(value, kind)
    {
        return Err(format!("the setting {name} must be of type {kind}"));
    }

    let allowed = property.get("enum").and_then(Value::as_array);
    if allowed.is_some_and(|allowed| !allowed.contains(value)) {
        return Err(format!(
            "the setting {name} must be one of its listed values"
        ));
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SettingsCheck;

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string" },
                "every": { "type": "integer" },
                "unit": { "type": "string", "enum": ["s", "m"] }
            },
            "required": ["url"]
        })
    }

    #[test]
    fn accepts_values_that_match_the_schema() {
        let schema = schema();
        let check = SettingsCheck::new(&schema);

        assert_eq!(
            check.check(&json!({ "url": "https://x", "every": 5, "unit": "m" })),
            Ok(())
        );
    }

    #[test]
    fn refuses_unknown_wrongly_typed_missing_and_unlisted_values() {
        let schema = schema();
        let check = SettingsCheck::new(&schema);

        for values in [
            json!({ "url": "https://x", "color": "red" }),
            json!({ "url": 5 }),
            json!({ "url": "https://x", "every": 1.5 }),
            json!({ "every": 5 }),
            json!({ "url": "https://x", "unit": "h" }),
            json!(["url"]),
        ] {
            assert!(check.check(&values).is_err(), "{values}");
        }
    }
}
