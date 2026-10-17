use serde::Deserialize;
use synt_proto::OneLine;

use crate::manifest::{Capabilities, Description, ExtensionId};
use crate::remote::Listing;
use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ListingSeal {
    id: ExtensionId,
    name: OneLine,
    version: OneLine,
    description: Description,
    repository: Option<String>,
    sha256: Sha256,
    capabilities: Capabilities,
}

impl ListingSeal {
    pub(crate) fn covers(&self, listing: &Listing) -> bool {
        let manifest = listing.manifest();

        self.id == *manifest.id()
            && self.name == *manifest.name()
            && self.version == *manifest.version()
            && self.description == *manifest.description()
            && self.repository.as_deref() == manifest.repository()
            && self.sha256 == *listing.sha256()
            && self.capabilities == *manifest.capabilities()
    }
}
