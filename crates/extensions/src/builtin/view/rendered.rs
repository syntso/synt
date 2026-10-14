use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Rendered {
    pub extension: String,
    pub view: String,
    pub tree: Value,
}
