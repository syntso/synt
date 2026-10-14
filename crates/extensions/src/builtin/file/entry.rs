use serde::Serialize;

use crate::manifest::FileAccess;
use crate::picked::Picked;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FileEntry {
    pub extension: String,
    pub handle: String,
    pub name: String,
    pub access: FileAccess,
    pub path: String,
}

impl FileEntry {
    pub(crate) fn new(extension: &str, picked: Picked) -> Self {
        Self {
            extension: extension.to_owned(),
            handle: picked.handle,
            name: picked.name,
            access: picked.access,
            path: picked.path,
        }
    }
}
