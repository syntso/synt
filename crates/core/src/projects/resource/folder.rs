use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_platform::{Found, replace_file};

use super::{Resource, ResourceName};
use crate::CoreError;
use crate::projects::Text;

const EXTENSION: &str = "toml";

#[derive(Debug)]
pub(crate) struct ResourceFolder {
    path: PathBuf,
}

impl ResourceFolder {
    pub(crate) const NAME: &str = "resources";

    pub(crate) fn of(project: &Path) -> Self {
        Self {
            path: project.join(Self::NAME),
        }
    }

    pub(crate) fn prepare(project: &Path) -> Result<Self, CoreError> {
        let folder = Self::of(project);

        match fs::create_dir(&folder.path) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error.into()),
            _ => {}
        }

        folder.check()?;

        Ok(folder)
    }

    pub(crate) fn file_label(name: &str) -> String {
        format!("{}/{name}.{EXTENSION}", Self::NAME)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        let metadata = fs::symlink_metadata(&self.path).map_err(CoreError::Unreadable)?;

        if metadata.is_dir() {
            return Ok(());
        }

        Err(CoreError::LinkedFolder(self.path.clone()))
    }

    pub(crate) fn list(&self) -> Result<BTreeMap<String, PathBuf>, CoreError> {
        self.check()?;

        let mut found = BTreeMap::new();

        for entry in fs::read_dir(&self.path).map_err(CoreError::Unreadable)? {
            let entry = entry.map_err(CoreError::Unreadable)?;
            let is_file = entry.file_type().map_err(CoreError::Unreadable)?.is_file();
            let path = entry.path();

            if let Some(name) = Self::name_of(&path).filter(|_| is_file) {
                found.insert(name, path);
            }
        }

        Ok(found)
    }

    pub(crate) fn read(path: &Path) -> Result<Resource, CoreError> {
        let text = Text::read(path).map_err(CoreError::Unreadable)?;

        Resource::from_toml(text.as_str())
    }

    pub(crate) fn write(&self, name: &ResourceName, resource: &Resource) -> Result<(), CoreError> {
        let text = resource.to_toml()?;

        self.check()?;
        replace_file(&self.path_of(name.as_str()), text.as_bytes()).map_err(CoreError::Unreadable)
    }

    pub(crate) fn remove(&self, name: &ResourceName) -> Result<(), CoreError> {
        fs::remove_file(self.path_of(name.as_str())).found()?;

        Ok(())
    }

    fn path_of(&self, name: &str) -> PathBuf {
        self.path.join(format!("{name}.{EXTENSION}"))
    }

    fn name_of(path: &Path) -> Option<String> {
        if path.extension()? != EXTENSION {
            return None;
        }

        let stem = path.file_stem()?.to_str()?;

        (!stem.starts_with('.')).then(|| stem.to_owned())
    }
}
