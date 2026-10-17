use std::fs;
use std::path::{Path, PathBuf};

use super::Package;
use super::package::{COMPONENT_FILE, MANIFEST_FILE};
use crate::ExtensionError;
use crate::manifest::Manifest;

const BUILD_OUTPUT: [&str; 3] = ["target", "wasm32-wasip2", "release"];
const LARGEST_COMPONENT: u64 = 32 * 1024 * 1024;

pub(crate) struct LocalBuild {
    folder: PathBuf,
}

impl LocalBuild {
    pub(crate) fn new(folder: &Path) -> Self {
        Self {
            folder: folder.to_owned(),
        }
    }

    pub(crate) fn read(&self) -> Result<Package, ExtensionError> {
        let manifest = self.manifest()?;
        let component = self.component()?;

        Self::check_size(&component)?;
        let wasm = fs::read(&component)?;

        Ok(Package::new(manifest, wasm, false))
    }

    fn manifest(&self) -> Result<Manifest, ExtensionError> {
        let path = self.folder.join(MANIFEST_FILE);
        let text = fs::read_to_string(&path)
            .map_err(|source| ExtensionError::BuildManifest { path, source })?;

        Manifest::parse(&text)
    }

    fn component(&self) -> Result<PathBuf, ExtensionError> {
        let beside = self.folder.join(COMPONENT_FILE);
        if beside.is_file() {
            return Ok(beside);
        }

        let output = BUILD_OUTPUT
            .iter()
            .fold(self.folder.clone(), |path, part| path.join(part));

        match Self::wasm_files(&output).as_slice() {
            [only] => Ok(only.clone()),
            [] => Err(ExtensionError::NoComponent {
                folder: self.folder.clone(),
                output,
            }),
            _ => Err(ExtensionError::SeveralComponents { output, beside }),
        }
    }

    fn check_size(component: &Path) -> Result<(), ExtensionError> {
        let size = fs::metadata(component)?.len();

        if size > LARGEST_COMPONENT {
            return Err(ExtensionError::ComponentTooLarge {
                path: component.to_owned(),
                size,
                limit: LARGEST_COMPONENT,
            });
        }

        Ok(())
    }

    fn wasm_files(folder: &Path) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(folder) else {
            return Vec::new();
        };

        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().is_some_and(|suffix| suffix == "wasm")
            })
            .collect()
    }
}
