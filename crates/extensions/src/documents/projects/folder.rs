use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFolder {
    id: String,
    path: PathBuf,
}

impl ProjectFolder {
    pub fn new(id: String, path: PathBuf) -> Self {
        Self { id, path }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}
