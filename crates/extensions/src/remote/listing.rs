mod release_file;

use serde::{Deserialize, Serialize};

pub(crate) use release_file::ReleaseFile;

use crate::manifest::Manifest;
use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Listing {
    #[serde(flatten)]
    manifest: Manifest,

    file: ReleaseFile,
    sha256: Sha256,
    signature: String,
}

impl Listing {
    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub(crate) fn file(&self) -> &ReleaseFile {
        &self.file
    }

    pub(crate) fn sha256(&self) -> &Sha256 {
        &self.sha256
    }

    pub(crate) fn signature(&self) -> &str {
        &self.signature
    }

    pub(crate) fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        let manifest = &self.manifest;

        [
            manifest.id().as_str(),
            manifest.name().as_str(),
            manifest.description().as_str(),
        ]
        .iter()
        .any(|field| field.to_lowercase().contains(&query))
    }
}
