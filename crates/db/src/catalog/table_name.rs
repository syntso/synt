use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TableName {
    pub schema: Option<String>,
    pub name: String,
    pub kind: String,
}
