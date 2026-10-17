use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Login(String);

impl Login {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        let refused = |character: char| {
            character.is_whitespace() || character.is_control() || character == '@'
        };

        if text.is_empty() || text.contains(refused) {
            return Err(CoreError::NotALogin(text.to_owned()));
        }

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Login {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text)
    }
}

impl From<Login> for String {
    fn from(login: Login) -> Self {
        login.0
    }
}

#[cfg(test)]
mod tests {
    use super::Login;

    #[test]
    fn a_login_is_one_plain_word_without_a_host() {
        assert_eq!(Login::parse("deploy").unwrap().as_str(), "deploy");

        for text in ["", "de ploy", "deploy@web"] {
            assert!(Login::parse(text).is_err(), "{text:?}");
        }
    }
}
