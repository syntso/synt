use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::runbooks::RunbookError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct OneLine(String);

impl OneLine {
    const LONGEST: usize = 200;

    pub(crate) fn new(text: impl Into<String>) -> Result<Self, RunbookError> {
        let text = text.into();
        let fits = !text.trim().is_empty() && text.len() <= Self::LONGEST;

        if !fits || text.contains(char::is_control) {
            return Err(RunbookError::NotOneLine {
                text,
                longest: Self::LONGEST,
            });
        }

        Ok(Self(text))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OneLine {
    type Error = RunbookError;

    fn try_from(text: String) -> Result<Self, RunbookError> {
        Self::new(text)
    }
}

impl From<OneLine> for String {
    fn from(line: OneLine) -> Self {
        line.0
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
    fn a_line_holds_text_without_line_breaks() {
        assert_eq!(OneLine::new("Restart API").unwrap().as_str(), "Restart API");

        assert!(OneLine::new("").is_err());
        assert!(OneLine::new("  ").is_err());
        assert!(OneLine::new("two\nlines").is_err());
        assert!(OneLine::new("a".repeat(201)).is_err());
    }
}
