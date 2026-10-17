use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ProtoError;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
pub struct OneLine(String);

impl OneLine {
    pub const LONGEST: usize = 200;

    pub fn new(text: impl Into<String>) -> Result<Self, ProtoError> {
        let text = text.into();
        let blank = text.trim().is_empty();

        if blank || text.len() > Self::LONGEST || text.contains(char::is_control) {
            return Err(ProtoError::NotOneLine {
                text,
                longest: Self::LONGEST,
            });
        }

        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OneLine {
    type Error = ProtoError;

    fn try_from(text: String) -> Result<Self, ProtoError> {
        Self::new(text)
    }
}

impl From<OneLine> for String {
    fn from(line: OneLine) -> Self {
        line.0
    }
}

impl Borrow<str> for OneLine {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for OneLine {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::OneLine;

    #[test]
    fn a_line_holds_visible_text_without_breaks() {
        assert_eq!(OneLine::new("Restart API").unwrap().as_str(), "Restart API");

        for text in ["", "   ", "two\nlines", &"x".repeat(201)] {
            assert!(OneLine::new(text).is_err(), "{text:?}");
        }
    }
}
