use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ExtensionError;
use crate::manifest::checks::refused;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema: Value,
}

impl Settings {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        let is_object = self.schema.get("type").and_then(Value::as_str) == Some("object");
        let has_properties = self.schema.get("properties").is_some_and(Value::is_object);

        if is_object && has_properties {
            return Ok(());
        }

        Err(refused(
            "settings.schema must be a JSON Schema of type object with properties",
        ))
    }
}
