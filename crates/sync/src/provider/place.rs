use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{FolderProvider, SyncProvider};
use crate::SyncError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    Folder(PathBuf),
}

impl Place {
    pub fn connect(&self) -> Result<Arc<dyn SyncProvider>, SyncError> {
        match self {
            Self::Folder(path) => Ok(Arc::new(FolderProvider::open(path)?)),
        }
    }
}
