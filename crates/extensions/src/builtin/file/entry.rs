use serde::Serialize;
use serde_json::{Value, json};

use crate::files::{FileAccess, FileHandle, Picked};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FileEntry {
    extension: String,
    handle: FileHandle,
    name: String,
    access: FileAccess,
    path: String,
}

impl FileEntry {
    pub(crate) fn new(extension: &str, picked: &Picked) -> Self {
        Self {
            extension: extension.to_owned(),
            handle: picked.handle().clone(),
            name: picked.name().to_owned(),
            access: picked.access(),
            path: picked.path().to_string_lossy().into_owned(),
        }
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "extension": self.extension,
            "handle": self.handle,
            "name": self.name,
            "access": self.access,
        })
    }
}
