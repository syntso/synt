use std::fmt;

use serde::Serialize;

use super::Name;
use crate::error::NameError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ResourceRef(Name);

impl ResourceRef {
    pub fn new(reference: impl Into<String>) -> Result<Self, NameError> {
        Name::new(reference).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ResourceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
