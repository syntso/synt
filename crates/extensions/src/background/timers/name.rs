use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use crate::ExtensionError;

const LONGEST: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TimerName(String);

impl TryFrom<&str> for TimerName {
    type Error = ExtensionError;

    fn try_from(name: &str) -> Result<Self, ExtensionError> {
        let plain = name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character));

        if name.is_empty() || name.len() > LONGEST || !plain {
            return Err(ExtensionError::TimerName { longest: LONGEST });
        }

        Ok(Self(name.to_owned()))
    }
}

impl Borrow<str> for TimerName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<TimerName> for String {
    fn from(name: TimerName) -> Self {
        name.0
    }
}

impl Display for TimerName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
