use std::fmt;

use serde::Serialize;

use crate::error::NameError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Name(String);

impl Name {
    pub const MAX_LEN: usize = 256;

    pub fn new(text: impl Into<String>) -> Result<Self, NameError> {
        let text = text.into();

        if text.is_empty() {
            return Err(NameError::Empty);
        }
        if text.len() > Self::MAX_LEN {
            return Err(NameError::TooLong { len: text.len() });
        }
        if text.chars().any(char::is_control) {
            return Err(NameError::ControlCharacter);
        }

        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Name;
    use crate::error::NameError;

    #[test]
    fn accepts_ordinary_names() {
        assert_eq!(
            Name::new("ssh.exec").map(|name| name.to_string()),
            Ok("ssh.exec".to_owned())
        );
    }

    #[test]
    fn refuses_empty_names() {
        assert_eq!(Name::new(""), Err(NameError::Empty));
    }

    #[test]
    fn refuses_names_over_the_limit() {
        let long = "a".repeat(Name::MAX_LEN + 1);
        assert_eq!(
            Name::new(long),
            Err(NameError::TooLong {
                len: Name::MAX_LEN + 1
            })
        );
    }

    #[test]
    fn refuses_control_characters() {
        assert_eq!(
            Name::new("api\n#2 forged"),
            Err(NameError::ControlCharacter)
        );
    }
}
