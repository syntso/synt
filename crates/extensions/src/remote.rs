mod fetcher;
mod index;
mod listing;
mod seal;
mod trusted_key;

use synt_journal::Timestamp;

pub(crate) use index::Index;
pub(crate) use listing::Listing;

use fetcher::Fetcher;
use trusted_key::TrustedKey;

use crate::ExtensionError;
use crate::install::Package;

const REGISTRY: &str = "https://github.com/syntso/extensions/releases/latest/download";
const REGISTRY_KEY: &str = include_str!("../registry.pub");
const INDEX_FILE: &str = "index.json";
const INDEX_SIGNATURE_FILE: &str = "index.json.minisig";
const SIGNATURE_LIMIT: u64 = 64 * 1024;
const INDEX_LIMIT: u64 = 4 * 1024 * 1024;
const COMPONENT_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Remote {
    fetcher: Fetcher,
    key: TrustedKey,
}

impl Remote {
    pub fn official() -> Result<Self, ExtensionError> {
        Ok(Self {
            fetcher: Fetcher::new(REGISTRY),
            key: TrustedKey::decode(REGISTRY_KEY)?,
        })
    }

    pub fn new(base: &str, public_key: &str) -> Result<Self, ExtensionError> {
        Ok(Self {
            fetcher: Fetcher::new(base),
            key: TrustedKey::from_base64(public_key)?,
        })
    }

    pub(crate) fn index(&self, now: Timestamp) -> Result<Index, ExtensionError> {
        let bytes = self.fetcher.get(INDEX_FILE, INDEX_LIMIT)?;
        let signature = self.fetcher.get(INDEX_SIGNATURE_FILE, SIGNATURE_LIMIT)?;

        let seal = self.key.check_index(&bytes, &signature, now)?;

        Index::parse(&bytes, seal.serial())
    }

    pub(crate) fn download(&self, listing: &Listing) -> Result<Package, ExtensionError> {
        let wasm = self.fetcher.get(listing.file().as_str(), COMPONENT_LIMIT)?;

        self.key.check(&wasm, listing)?;

        Ok(Package::new(listing.manifest().clone(), wasm, true))
    }
}

#[cfg(test)]
mod tests {
    use super::Remote;

    #[test]
    fn the_built_in_key_is_valid() {
        assert!(Remote::official().is_ok());
    }
}
