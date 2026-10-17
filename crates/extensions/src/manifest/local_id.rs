use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ExtensionError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocalId(String);

impl LocalId {
    pub(crate) const LONGEST: usize = 40;

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for LocalId {
    type Error = ExtensionError;

    fn try_from(text: String) -> Result<Self, ExtensionError> {
        let starts_with_letter = text.starts_with(|first: char| first.is_ascii_lowercase());
        let plain = text.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || "-_".contains(character)
        });

        if !starts_with_letter || !plain || text.len() > Self::LONGEST {
            return Err(ExtensionError::LocalId(text));
        }

        Ok(Self(text))
    }
}

impl From<LocalId> for String {
    fn from(value: LocalId) -> Self {
        value.0
    }
}

impl Borrow<str> for LocalId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for LocalId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::LocalId;

    #[test]
    fn local_ids_are_plain() {
        assert!(LocalId::try_from("load-chart_2".to_owned()).is_ok());

        for id in ["", "Load", "2load", "a.b", "a b"] {
            assert!(LocalId::try_from(id.to_owned()).is_err(), "{id:?}");
        }
    }
}
