use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{Name, ProtoError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActionId(Name);

impl ActionId {
    pub fn new(id: impl Into<String>) -> Result<Self, ProtoError> {
        Name::new(id).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for ActionId {
    type Error = ProtoError;

    fn try_from(id: String) -> Result<Self, ProtoError> {
        Self::new(id)
    }
}

impl From<ActionId> for String {
    fn from(id: ActionId) -> Self {
        id.0.into()
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
