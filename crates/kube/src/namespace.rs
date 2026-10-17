mod scope;

pub use scope::NamespaceScope;

use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::KubeError;

const LONGEST: usize = 63;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct Namespace(String);

impl Namespace {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(text: &str) -> bool {
        let allowed = |character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        };

        !text.is_empty()
            && text.len() <= LONGEST
            && text.chars().all(allowed)
            && !text.starts_with('-')
            && !text.ends_with('-')
    }
}

impl TryFrom<String> for Namespace {
    type Error = KubeError;

    fn try_from(text: String) -> Result<Self, KubeError> {
        if !Self::is_valid(&text) {
            return Err(KubeError::invalid(format!(
                "namespace must be lowercase letters, digits and '-', at most {LONGEST} \
                 characters, got {text:?}"
            )));
        }

        Ok(Self(text))
    }
}

impl TryFrom<&str> for Namespace {
    type Error = KubeError;

    fn try_from(text: &str) -> Result<Self, KubeError> {
        Self::try_from(text.to_owned())
    }
}

impl From<Namespace> for String {
    fn from(namespace: Namespace) -> Self {
        namespace.0
    }
}

impl Display for Namespace {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Namespace;

    #[test]
    fn a_dns_label_is_a_namespace() {
        assert_eq!(
            Namespace::try_from("orders-2").unwrap().as_str(),
            "orders-2"
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for text in ["", "Orders", "-orders", "orders-", "a/b", &"a".repeat(64)] {
            assert!(Namespace::try_from(text).is_err(), "{text:?}");
        }
    }
}
