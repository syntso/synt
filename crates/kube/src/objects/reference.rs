use std::fmt::{self, Display, Formatter};

use serde::Serialize;
use serde_json::Value;

use super::Fields;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectRef {
    pub kind: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl ObjectRef {
    pub(crate) fn of(object: &Value) -> Self {
        Self {
            kind: object.text("kind").unwrap_or_default().to_owned(),
            name: object.metadata_text("name").unwrap_or_default().to_owned(),
            namespace: object.metadata_text("namespace").map(str::to_owned),
        }
    }
}

impl Display for ObjectRef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.namespace {
            Some(namespace) => write!(formatter, "{} {}/{}", self.kind, namespace, self.name),
            None => write!(formatter, "{} {}", self.kind, self.name),
        }
    }
}
