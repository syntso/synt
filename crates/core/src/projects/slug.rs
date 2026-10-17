use std::fmt;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Personal, ProjectId};
use crate::CoreError;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectSlug(String);

impl ProjectSlug {
    pub(crate) fn of_folder(path: &Path) -> Result<Self, CoreError> {
        let slug = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty() && !name.contains(char::is_control))
            .ok_or_else(|| CoreError::UnnamedFolder(path.to_owned()))?;

        if Personal::is_named(slug) {
            return Err(CoreError::PersonalSlug(slug.to_owned()));
        }

        if ProjectId::parse(slug).is_ok() {
            return Err(CoreError::IdShapedSlug(slug.to_owned()));
        }

        Ok(Self(slug.to_owned()))
    }

    pub(crate) fn named(text: &str) -> Result<Self, CoreError> {
        if text.is_empty()
            || text.contains(|character: char| character.is_control() || character == '/')
        {
            return Err(CoreError::ProjectNotOpen(text.to_owned()));
        }

        Ok(Self(text.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProjectSlug {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::named(&text)
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
