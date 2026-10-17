use super::Package;
use crate::ExtensionError;
use crate::manifest::Manifest;
use crate::sha256::Sha256;
use crate::shelf::Installed;

pub(crate) struct InstalledFiles {
    manifest: Manifest,
    manifest_sha256: Sha256,
    wasm: Vec<u8>,
}

impl InstalledFiles {
    pub(super) fn new(manifest: Manifest, manifest_sha256: Sha256, wasm: Vec<u8>) -> Self {
        Self {
            manifest,
            manifest_sha256,
            wasm,
        }
    }

    pub(crate) fn into_manifest(self) -> Manifest {
        self.manifest
    }

    pub(crate) fn package_for(self, entry: &Installed) -> Result<Package, ExtensionError> {
        let manifest_sha256 = self.manifest_sha256;
        let package = Package::new(self.manifest, self.wasm, entry.signed());

        let unchanged = package.sha256() == entry.sha256()
            && manifest_sha256 == *entry.manifest_sha256()
            && package.manifest().id() == entry.id();

        if !unchanged {
            return Err(ExtensionError::FilesChanged(entry.id().to_string()));
        }

        Ok(package)
    }
}
