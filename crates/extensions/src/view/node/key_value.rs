use serde::Deserialize;
use serde_json::Value;

use crate::ExtensionError;

#[derive(Debug, Deserialize)]
pub(in crate::view) struct KeyValue {
    #[serde(rename = "key")]
    _key: String,

    value: Value,
}

impl KeyValue {
    pub(in crate::view) fn check(&self) -> Result<(), ExtensionError> {
        if self.value.is_array() || self.value.is_object() {
            return Err(ExtensionError::KeyValue);
        }

        Ok(())
    }
}
