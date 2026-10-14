use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cursor(String);

impl Cursor {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}
