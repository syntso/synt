use std::fs;
use std::io;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use synt_platform::{read_plain_file, replace_file};

use super::AtPath;
use crate::SyncError;

const LARGEST_FILE: u64 = 4 * 1024 * 1024;

pub(super) struct JsonFile {
    path: PathBuf,
}

impl JsonFile {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) fn read<T: DeserializeOwned>(&self) -> Result<Option<T>, SyncError> {
        let bytes = match read_plain_file(&self.path, LARGEST_FILE) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            read => read.at(&self.path)?,
        };

        serde_json::from_slice(&bytes).map(Some).map_err(|error| {
            SyncError::provider(format!("{} is damaged: {error}", self.path.display()))
        })
    }

    pub(super) fn write(&self, value: &impl Serialize) -> Result<(), SyncError> {
        let bytes = serde_json::to_vec_pretty(value)
            .map_err(|error| SyncError::provider(format!("cannot encode: {error}")))?;

        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder).at(folder)?;
        }

        replace_file(&self.path, &bytes).at(&self.path)
    }

    pub(super) fn remove(&self) -> Result<(), SyncError> {
        match fs::remove_file(&self.path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other.at(&self.path),
        }
    }
}
