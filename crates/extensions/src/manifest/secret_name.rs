use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ManifestError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SecretName(String);

impl SecretName {
    pub(crate) const LONGEST: usize = 64;
}

impl TryFrom<String> for SecretName {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        let plain_segment = |segment: &str| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
        };

        if text.is_empty() || text.len() > Self::LONGEST || !text.split('/').all(plain_segment) {
            return Err(ManifestError::SecretName(text));
        }

        Ok(Self(text))
    }
}

impl From<SecretName> for String {
    fn from(value: SecretName) -> Self {
        value.0
    }
}

impl Borrow<str> for SecretName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Display for SecretName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SecretName;

    #[test]
    fn secret_names_match_the_vault_rules() {
        assert!(SecretName::try_from("grafana/token".to_owned()).is_ok());

        for name in ["", "a//b", "../x", "a.b", "a b", "/a"] {
            assert!(SecretName::try_from(name.to_owned()).is_err(), "{name:?}");
        }
    }
}
