use std::fmt;

use serde::Serialize;

use super::Name;
use crate::error::NameError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ActionId(Name);

impl ActionId {
    pub fn new(id: impl Into<String>) -> Result<Self, NameError> {
        Name::new(id).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
