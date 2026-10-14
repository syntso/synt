use std::path::PathBuf;

use crate::id::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenFolder {
    pub(crate) id: ProjectId,
    pub(crate) path: PathBuf,
}
