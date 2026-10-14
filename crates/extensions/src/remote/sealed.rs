use serde::Deserialize;

use super::Listing;
use crate::manifest::Capabilities;
use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct Sealed {
    id: String,
    name: String,
    version: String,
    description: String,
    repository: Option<String>,
    sha256: Sha256,
    capabilities: Capabilities,
}

impl Sealed {
    pub(super) fn covers(&self, listing: &Listing) -> bool {
        self.id == listing.id
            && self.name == listing.name
            && self.version == listing.version
            && self.description == listing.description
            && self.repository == listing.repository
            && self.sha256 == listing.sha256
            && self.capabilities == listing.capabilities
    }
}
