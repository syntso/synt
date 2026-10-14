use crate::digest::sha256;
use crate::manifest::Manifest;

pub(super) const MANIFEST_FILE: &str = "extension.toml";
pub(super) const COMPONENT_FILE: &str = "extension.wasm";

#[derive(Debug, Clone)]
pub(crate) struct Package {
    pub(crate) manifest: Manifest,
    pub(crate) wasm: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) signed: bool,
}

impl Package {
    pub(crate) fn new(manifest: Manifest, wasm: Vec<u8>, signed: bool) -> Self {
        let sha256 = sha256(&wasm);

        Self {
            manifest,
            wasm,
            sha256,
            signed,
        }
    }
}
