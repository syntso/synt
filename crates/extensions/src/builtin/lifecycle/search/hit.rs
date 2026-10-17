use serde::Serialize;

use crate::manifest::{Capabilities, Description, ExtensionId, OneLine};
use crate::remote::Listing;
use crate::shelf::Installed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SearchHit {
    id: ExtensionId,
    name: OneLine,
    version: OneLine,
    description: Description,
    repository: Option<String>,
    capabilities: Capabilities,
    installed: Option<OneLine>,
}

impl SearchHit {
    pub(crate) fn new(listing: &Listing, installed: Option<&Installed>) -> Self {
        let manifest = listing.manifest();

        Self {
            id: manifest.id().clone(),
            name: manifest.name().clone(),
            version: manifest.version().clone(),
            description: manifest.description().clone(),
            repository: manifest.repository().map(str::to_owned),
            capabilities: manifest.capabilities().clone(),
            installed: installed.map(|entry| entry.version().clone()),
        }
    }
}
