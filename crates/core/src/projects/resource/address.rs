use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Address(String);

impl Address {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        let refused = |character: char| {
            character.is_whitespace() || character.is_control() || character == '@'
        };

        if text.is_empty() || text.contains(refused) {
            return Err(CoreError::NotAnAddress(text.to_owned()));
        }

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Address {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text)
    }
}

impl From<Address> for String {
    fn from(address: Address) -> Self {
        address.0
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn an_address_is_one_plain_word() {
        assert_eq!(Address::parse("10.0.3.3").unwrap().as_str(), "10.0.3.3");

        for text in ["", "a b", "root@db", "db\n"] {
            assert!(Address::parse(text).is_err(), "{text:?}");
        }
    }
}
