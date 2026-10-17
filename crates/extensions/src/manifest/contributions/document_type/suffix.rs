use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ExtensionError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Suffix(String);

impl Suffix {
    pub(crate) const LONGEST: usize = 16;

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Suffix {
    type Error = ExtensionError;

    fn try_from(text: String) -> Result<Self, ExtensionError> {
        let plain = text
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit());

        if text.is_empty() || text.len() > Self::LONGEST || !plain {
            return Err(ExtensionError::Suffix(text));
        }

        Ok(Self(text))
    }
}

impl From<Suffix> for String {
    fn from(suffix: Suffix) -> Self {
        suffix.0
    }
}

impl Display for Suffix {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
