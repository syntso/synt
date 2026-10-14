use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SettingsValues {
    pub extension: String,
    pub schema: Value,
    pub values: Value,
}
