use serde::Serialize;

use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DocumentInfo {
    project: String,
    path: String,
    kind: String,
    revision: Sha256,
    size: u64,
}

impl DocumentInfo {
    pub(super) fn new(project: String, path: String, kind: String, body: &[u8]) -> Self {
        Self {
            project,
            path,
            kind,
            revision: Sha256::of(body),
            size: u64::try_from(body.len()).unwrap_or(u64::MAX),
        }
    }

    pub(crate) fn project(&self) -> &str {
        &self.project
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }

    pub(crate) fn revision(&self) -> &Sha256 {
        &self.revision
    }

    pub(crate) fn size(&self) -> u64 {
        self.size
    }
}
