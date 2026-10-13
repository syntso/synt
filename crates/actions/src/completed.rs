use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Completed {
    pub value: Value,
    pub summary: Value,
}
