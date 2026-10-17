use serde::Deserialize;

use super::Listing;
use crate::ExtensionError;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Index {
    extensions: Vec<Listing>,

    #[serde(skip)]
    serial: u64,
}

impl Index {
    pub(crate) fn parse(bytes: &[u8], serial: u64) -> Result<Self, ExtensionError> {
        let index: Self = serde_json::from_slice(bytes).map_err(ExtensionError::Index)?;

        Ok(Self { serial, ..index })
    }

    pub(crate) fn serial(&self) -> u64 {
        self.serial
    }

    pub(crate) fn listings(&self) -> &[Listing] {
        &self.extensions
    }

    pub(crate) fn find(&self, id: &str) -> Option<&Listing> {
        self.extensions
            .iter()
            .find(|listing| listing.manifest().id().as_str() == id)
    }
}
