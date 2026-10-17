use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ManifestError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OneLine(String);

impl OneLine {
    pub(crate) const LONGEST: usize = 200;

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OneLine {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        let blank = text.trim().is_empty();

        if blank || text.len() > Self::LONGEST || text.contains(char::is_control) {
            return Err(ManifestError::NotOneLine(text));
        }

        Ok(Self(text))
    }
}

impl From<OneLine> for String {
    fn from(value: OneLine) -> Self {
        value.0
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
    fn one_line_holds_visible_text_without_breaks() {
        assert!(OneLine::try_from("Greet a host".to_owned()).is_ok());

        for text in ["", "   ", "two\nlines", &"x".repeat(201)] {
            assert!(OneLine::try_from(text.to_owned()).is_err(), "{text:?}");
        }
    }
}
