use std::fs;
use std::path::{Path, PathBuf};

use crate::ExtensionError;
use crate::manifest::Manifest;
use crate::package::{COMPONENT_FILE, MANIFEST_FILE, Package};

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
        let manifest_path = self.folder.join(MANIFEST_FILE);
        let text = fs::read_to_string(&manifest_path).map_err(|error| {
            ExtensionError::Refused(format!("{}: {error}", manifest_path.display()))
        })?;
        let manifest = Manifest::parse(&text)?;

        let component = self.component()?;
        let size = fs::metadata(&component)?.len();
        if size > LARGEST_COMPONENT {
            return Err(ExtensionError::Refused(format!(
                "{} is {size} bytes; the limit is {LARGEST_COMPONENT}",
                component.display()
            )));
        }

        let wasm = fs::read(&component)?;
        Ok(Package::new(manifest, wasm, false))
    }

    fn component(&self) -> Result<PathBuf, ExtensionError> {
        let beside = self.folder.join(COMPONENT_FILE);
        if beside.is_file() {
            return Ok(beside);
        }

        let output: PathBuf = BUILD_OUTPUT
            .iter()
            .fold(self.folder.clone(), |path, part| path.join(part));
        let built = wasm_files(&output);

        match built.as_slice() {
            [only] => Ok(only.clone()),
            [] => Err(ExtensionError::Refused(format!(
                "no {COMPONENT_FILE} in {} and no build in {}; run cargo build --release --target \
                 wasm32-wasip2 first",
                self.folder.display(),
                output.display()
            ))),
            _ => Err(ExtensionError::Refused(format!(
                "{} holds several .wasm files; put the one to install at {}",
                output.display(),
                beside.display()
            ))),
        }
    }
}

fn wasm_files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "wasm")
        })
        .collect()
}
