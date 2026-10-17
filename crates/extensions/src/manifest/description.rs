use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ManifestError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Description(String);

impl Description {
    pub(crate) const LONGEST: usize = 1000;

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Description {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        if text.len() > Self::LONGEST || text.contains(char::is_control) {
            return Err(ManifestError::Description);
        }

        Ok(Self(text))
    }
}

impl From<Description> for String {
    fn from(description: Description) -> Self {
        description.0
    }
}

impl Display for Description {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
