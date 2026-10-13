use serde_json::Value;
use synt_journal::Name;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionCall {
    pub extension: Name,
    pub action: String,
    pub target: Option<String>,
    pub params: Value,
}
