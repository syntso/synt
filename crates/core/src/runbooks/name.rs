use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::RunbookError;
use crate::projects::ResourceName;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct RunbookName(String);

impl RunbookName {
    pub(crate) fn new(name: impl Into<String>) -> Result<Self, RunbookError> {
        let name = name.into();

        ResourceName::parse(&name).map_err(|error| RunbookError::BadName(Box::new(error)))?;

        Ok(Self(name))
    }
}

impl TryFrom<String> for RunbookName {
    type Error = RunbookError;

    fn try_from(name: String) -> Result<Self, RunbookError> {
        Self::new(name)
    }
}

impl From<RunbookName> for String {
    fn from(name: RunbookName) -> Self {
        name.0
    }
}

impl Display for RunbookName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::RunbookName;

    #[test]
    fn a_name_is_a_plain_file_stem() {
        assert_eq!(
            RunbookName::new("deploy-2").unwrap().to_string(),
            "deploy-2"
        );

        assert!(RunbookName::new("").is_err());
        assert!(RunbookName::new("../etc").is_err());
        assert!(RunbookName::new(".hidden").is_err());
    }
}
