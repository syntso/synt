use serde::Deserialize;

use super::Listing;
use crate::ExtensionError;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Index {
    pub(crate) extensions: Vec<Listing>,

    #[serde(skip)]
    pub(crate) serial: u64,
}

impl Index {
    pub(crate) fn parse(bytes: &[u8], serial: u64) -> Result<Self, ExtensionError> {
        let index: Self = serde_json::from_slice(bytes)
            .map_err(|error| ExtensionError::Index(error.to_string()))?;

        Ok(Self { serial, ..index })
    }

    pub(crate) fn find(&self, id: &str) -> Option<&Listing> {
        self.extensions.iter().find(|listing| listing.id == id)
    }
}
