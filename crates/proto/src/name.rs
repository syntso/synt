use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ProtoError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Name(String);

impl Name {
    pub const MAX_LEN: usize = 256;

    pub fn new(text: impl Into<String>) -> Result<Self, ProtoError> {
        let text = text.into();

        if text.is_empty() {
            return Err(ProtoError::EmptyName);
        }
        if text.len() > Self::MAX_LEN {
            return Err(ProtoError::LongName { len: text.len() });
        }
        if text.chars().any(char::is_control) {
            return Err(ProtoError::ControlCharacter);
        }

        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Name {
    type Error = ProtoError;

    fn try_from(text: String) -> Result<Self, ProtoError> {
        Self::new(text)
    }
}

impl From<Name> for String {
    fn from(name: Name) -> Self {
        name.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Name;
    use crate::ProtoError;

    #[test]
    fn accepts_ordinary_names() {
        assert_eq!(Name::new("ssh.exec").unwrap().to_string(), "ssh.exec");
    }

    #[test]
    fn refuses_empty_names() {
        assert!(matches!(Name::new(""), Err(ProtoError::EmptyName)));
    }

    #[test]
    fn refuses_names_over_the_limit() {
        let long = "a".repeat(Name::MAX_LEN + 1);

        assert!(matches!(
            Name::new(long),
            Err(ProtoError::LongName { len }) if len == Name::MAX_LEN + 1
        ));
    }

    #[test]
    fn refuses_control_characters() {
        assert!(matches!(
            Name::new("api\n#2 forged"),
            Err(ProtoError::ControlCharacter)
        ));
    }

    #[test]
    fn is_checked_when_read_from_json() {
        assert!(serde_json::from_value::<Name>(json!("")).is_err());
        assert_eq!(
            serde_json::from_value::<Name>(json!("deploy"))
                .unwrap()
                .as_str(),
            "deploy"
        );
    }
}
