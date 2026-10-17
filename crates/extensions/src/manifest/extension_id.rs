use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ManifestError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ExtensionId(String);

impl ExtensionId {
    pub(crate) const LONGEST: usize = 40;

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ExtensionId {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        let starts_with_letter = text.starts_with(|first: char| first.is_ascii_lowercase());
        let plain = text.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });

        if !starts_with_letter || !plain || text.len() > Self::LONGEST {
            return Err(ManifestError::ExtensionId(text));
        }

        Ok(Self(text))
    }
}

impl From<ExtensionId> for String {
    fn from(value: ExtensionId) -> Self {
        value.0
    }
}

impl Borrow<str> for ExtensionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for ExtensionId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ExtensionId;

    #[test]
    fn ids_are_lowercase_words_joined_by_dashes() {
        assert!(ExtensionId::try_from("uptime-2".to_owned()).is_ok());

        for id in ["Hello", "1st", "with space", "dot.ted", "under_score", ""] {
            assert!(ExtensionId::try_from(id.to_owned()).is_err(), "{id}");
        }
    }
}
