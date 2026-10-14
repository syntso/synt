use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::package::{COMPONENT_FILE, MANIFEST_FILE};
use super::{InstalledFiles, Package};
use crate::ExtensionError;
use crate::digest::sha256;
use crate::manifest::Manifest;

const STAGING: &str = ".staging";

#[derive(Debug, Clone)]
pub(crate) struct InstallFolder {
    root: PathBuf,
}

impl InstallFolder {
    pub(crate) fn open(root: PathBuf) -> Result<Self, ExtensionError> {
        fs::create_dir_all(&root)?;

        Ok(Self { root })
    }

    pub(crate) fn store(&self, package: &Package) -> Result<String, ExtensionError> {
        let staging = self.root.join(STAGING).join(&package.manifest.id);
        clear(&staging)?;
        fs::create_dir_all(&staging)?;

        let manifest = package.manifest.to_toml()?;
        fs::write(staging.join(MANIFEST_FILE), &manifest)?;
        fs::write(staging.join(COMPONENT_FILE), &package.wasm)?;

        let place = self.place(&package.manifest.id);
        clear(&place)?;
        fs::rename(&staging, &place)?;

        Ok(sha256(manifest.as_bytes()))
    }

    pub(crate) fn read(&self, id: &str) -> Result<InstalledFiles, ExtensionError> {
        let place = self.place(id);

        let text = fs::read_to_string(place.join(MANIFEST_FILE))?;
        let manifest = Manifest::parse(&text)?;
        let wasm = fs::read(place.join(COMPONENT_FILE))?;

        Ok(InstalledFiles {
            manifest,
            manifest_sha256: sha256(text.as_bytes()),
            wasm,
        })
    }

    pub(crate) fn remove(&self, id: &str) -> Result<(), ExtensionError> {
        clear(&self.place(id))
    }

    fn place(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }
}

fn clear(path: &Path) -> Result<(), ExtensionError> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}
