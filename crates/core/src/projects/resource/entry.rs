use serde::Serialize;

use super::Resource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceEntry {
    pub name: String,

    #[serde(flatten)]
    pub resource: Resource,
}

impl ResourceEntry {
    pub(crate) fn new(name: &str, resource: Resource) -> Self {
        Self {
            name: name.to_owned(),
            resource,
        }
    }
}
