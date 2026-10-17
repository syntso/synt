use std::fs::FileType;
use std::io;
use std::path::Path;

use crate::SyncError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Entry {
    Missing,
    Folder,
    File,
    Other,
}

impl Entry {
    pub(super) fn at(path: &Path) -> Result<Self, SyncError> {
        match path.symlink_metadata() {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::Missing),
            metadata => Ok(Self::of(metadata?.file_type())),
        }
    }

    pub(super) fn of(file_type: FileType) -> Self {
        if file_type.is_dir() {
            return Self::Folder;
        }

        if file_type.is_file() {
            return Self::File;
        }

        Self::Other
    }
}
