use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{Name, ProtoError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceRef(Name);

impl ResourceRef {
    pub fn new(reference: impl Into<String>) -> Result<Self, ProtoError> {
        Name::new(reference).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for ResourceRef {
    type Error = ProtoError;

    fn try_from(reference: String) -> Result<Self, ProtoError> {
        Self::new(reference)
    }
}

impl From<ResourceRef> for String {
    fn from(reference: ResourceRef) -> Self {
        reference.0.into()
    }
}

impl fmt::Display for ResourceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
