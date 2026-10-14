use super::fetcher::Fetcher;
use super::{Index, Listing, TrustedKey};
use crate::ExtensionError;
use crate::install::Package;

const REGISTRY: &str = "https://github.com/syntso/extensions/releases/latest/download";
const REGISTRY_KEY: &str = include_str!("../../registry.pub");
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

    pub(crate) fn index(&self) -> Result<Index, ExtensionError> {
        let bytes = self.fetcher.get(INDEX_FILE, INDEX_LIMIT)?;
        let signature = self.fetcher.get(INDEX_SIGNATURE_FILE, SIGNATURE_LIMIT)?;

        let seal = self.key.check_index(&bytes, &signature)?;

        Index::parse(&bytes, seal.serial)
    }

    pub(crate) fn download(&self, listing: &Listing) -> Result<Package, ExtensionError> {
        if !is_plain_file_name(&listing.file) {
            return Err(ExtensionError::Index(format!(
                "{} names the file {:?}",
                listing.id, listing.file
            )));
        }

        let manifest = listing.manifest()?;
        manifest.check()?;

        let wasm = self.fetcher.get(&listing.file, COMPONENT_LIMIT)?;
        self.key.check(&wasm, listing)?;

        Ok(Package::new(manifest, wasm, true))
    }
}

fn is_plain_file_name(file: &str) -> bool {
    !file.is_empty()
        && file
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        && !file.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::{Remote, is_plain_file_name};

    #[test]
    fn the_built_in_key_is_valid() {
        assert!(Remote::official().is_ok());
    }

    #[test]
    fn file_names_stay_inside_the_release() {
        assert!(is_plain_file_name("uptime-0.1.0.wasm"));

        for name in ["", ".hidden", "../index.json", "a/b.wasm", "https://x"] {
            assert!(!is_plain_file_name(name), "{name}");
        }
    }
}
