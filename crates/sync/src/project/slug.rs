use std::fmt;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::SyncError;
use crate::id::ProjectId;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectSlug(String);

impl ProjectSlug {
    pub const PERSONAL: &str = "personal";

    pub fn of_folder(path: &Path) -> Result<Self, SyncError> {
        let slug = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty() && !name.contains(char::is_control))
            .ok_or_else(|| SyncError::UnnamedFolder(path.to_owned()))?;

        if Self::is_personal(slug) {
            return Err(SyncError::PersonalSlug(slug.to_owned()));
        }

        if ProjectId::parse(slug).is_ok() {
            return Err(SyncError::IdShapedSlug(slug.to_owned()));
        }

        Ok(Self(slug.to_owned()))
    }

    pub fn named(text: &str) -> Result<Self, SyncError> {
        if text.is_empty()
            || text.contains(|character: char| character.is_control() || character == '/')
        {
            return Err(SyncError::NotAProjectName(text.to_owned()));
        }

        Ok(Self(text.to_owned()))
    }

    pub fn is_personal(text: &str) -> bool {
        text.eq_ignore_ascii_case(Self::PERSONAL)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProjectSlug {
    type Error = SyncError;

    fn try_from(text: String) -> Result<Self, SyncError> {
        Self::named(&text)
    }
}

impl From<&ProjectId> for ProjectSlug {
    fn from(id: &ProjectId) -> Self {
        Self(id.get().to_owned())
    }
}

impl From<ProjectSlug> for String {
    fn from(slug: ProjectSlug) -> Self {
        slug.0
    }
}

impl fmt::Display for ProjectSlug {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
