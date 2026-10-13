use serde::Serialize;

use crate::manifest::Capabilities;
use crate::remote::Listing;
use crate::shelf::Installed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: Option<String>,
    pub capabilities: Capabilities,
    pub installed: Option<String>,
}

impl SearchHit {
    pub(crate) fn new(listing: &Listing, installed: Option<&Installed>) -> Self {
        Self {
            id: listing.id.clone(),
            name: listing.name.clone(),
            version: listing.version.clone(),
            description: listing.description.clone(),
            repository: listing.repository.clone(),
            capabilities: listing.capabilities.clone(),
            installed: installed.map(|entry| entry.version.clone()),
        }
    }
}
