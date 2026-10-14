use std::fmt::{self, Display, Formatter};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectRef {
    pub kind: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl Display for ObjectRef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.namespace {
            Some(namespace) => write!(formatter, "{} {}/{}", self.kind, namespace, self.name),
            None => write!(formatter, "{} {}", self.kind, self.name),
        }
    }
}
