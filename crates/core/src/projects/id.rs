use std::fmt;

use serde::{Deserialize, Serialize};

use crate::CoreError;

const BYTES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectId(String);

impl ProjectId {
    pub(crate) const LENGTH: usize = 2 * BYTES;

    pub(crate) fn generate() -> Result<Self, CoreError> {
        let mut bytes = [0_u8; BYTES];
        getrandom::fill(&mut bytes)?;

        Ok(Self(data_encoding::HEXLOWER.encode(&bytes)))
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        Self::fits(text).then(|| Self(text.to_owned()))
    }

    pub(crate) fn fits(text: &str) -> bool {
        let hex = |character: char| character.is_ascii_digit() || ('a'..='f').contains(&character);

        text.len() == Self::LENGTH && text.chars().all(hex)
    }
}

impl TryFrom<String> for ProjectId {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text).ok_or(CoreError::InvalidProjectId(text))
    }
}

impl From<ProjectId> for String {
    fn from(id: ProjectId) -> Self {
        id.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
