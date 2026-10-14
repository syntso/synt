use super::Package;
use crate::ExtensionError;
use crate::manifest::Manifest;
use crate::shelf::Installed;

pub(crate) struct InstalledFiles {
    pub(crate) manifest: Manifest,
    pub(super) manifest_sha256: String,
    pub(super) wasm: Vec<u8>,
}

impl InstalledFiles {
    pub(crate) fn package_for(self, entry: &Installed) -> Result<Package, ExtensionError> {
        let package = Package::new(self.manifest, self.wasm, entry.signed);

        let unchanged = package.sha256 == entry.sha256
            && self.manifest_sha256 == entry.manifest_sha256
            && package.manifest.id == entry.id;

        if !unchanged {
            return Err(ExtensionError::Refused(format!(
                "the installed files of {} changed since they were installed; install it again",
                entry.id
            )));
        }

        Ok(package)
    }
}
