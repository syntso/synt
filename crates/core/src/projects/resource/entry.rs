use serde::Serialize;

use super::Resource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceEntry {
    pub name: String,

    #[serde(flatten)]
    pub resource: Resource,
}
