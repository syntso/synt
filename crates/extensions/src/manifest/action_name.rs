use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use synt_journal::ActionId;

use crate::ManifestError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActionName(String);

impl ActionName {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn namespace(&self) -> &str {
        self.0.split('.').next().unwrap_or_default()
    }
}

impl TryFrom<String> for ActionName {
    type Error = ManifestError;

    fn try_from(text: String) -> Result<Self, ManifestError> {
        if let Err(source) = ActionId::new(text.as_str()) {
            return Err(ManifestError::ActionName {
                action: text,
                source,
            });
        }

        if text.contains(['*', '?']) {
            return Err(ManifestError::ActionPattern(text));
        }

        Ok(Self(text))
    }
}

impl From<ActionName> for String {
    fn from(name: ActionName) -> Self {
        name.0
    }
}

impl Display for ActionName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
