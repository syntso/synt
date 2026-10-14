use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use synt_platform::replace_file;

use super::{ProjectId, ProjectSlug, Text};
use crate::CoreError;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ProjectFile {
    #[serde(default)]
    id: Option<ProjectId>,

    #[serde(default)]
    name: Option<String>,
}

impl ProjectFile {
    pub(crate) const NAME: &str = "synt.toml";

    pub(crate) fn path_in(project: &Path) -> PathBuf {
        project.join(Self::NAME)
    }

    pub(crate) fn read(path: &Path) -> Result<Option<Self>, CoreError> {
        let text = match Text::read(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CoreError::Unreadable(error)),
        };

        Ok(Some(toml::from_str(text.as_str())?))
    }

    pub(crate) fn read_or_create(project: &Path, slug: &ProjectSlug) -> Result<Self, CoreError> {
        let path = Self::path_in(project);
        let invalid = |reason: CoreError| CoreError::ProjectFile {
            path: path.clone(),
            reason: Box::new(reason),
        };

        let existing = Self::read(&path).map_err(invalid)?;

        if let Some(file) = &existing
            && file.id.is_some()
        {
            return Ok(file.clone());
        }

        Self::create(&path, existing.as_ref(), slug)?;

        let written = Self::read(&path).map_err(invalid)?;

        written.ok_or_else(|| invalid(CoreError::Vanished))
    }

    pub(crate) fn id(&self) -> Option<&ProjectId> {
        self.id.as_ref()
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn create(path: &Path, existing: Option<&Self>, slug: &ProjectSlug) -> Result<(), CoreError> {
        let id = ProjectId::generate()?;
        let old = Text::read(path).map(Text::into_string).unwrap_or_default();

        let name_line = match existing.and_then(Self::name) {
            Some(_) => String::new(),
            None => format!("name = {}\n", toml::Value::String(slug.to_string())),
        };

        let text = format!("id = \"{id}\"\n{name_line}{old}");
        replace_file(path, text.as_bytes())?;

        Ok(())
    }
}
