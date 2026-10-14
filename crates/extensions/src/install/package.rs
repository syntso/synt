use crate::manifest::Manifest;
use crate::sha256::Sha256;

pub(super) const MANIFEST_FILE: &str = "extension.toml";
pub(super) const COMPONENT_FILE: &str = "extension.wasm";

#[derive(Debug, Clone)]
pub(crate) struct Package {
    pub(crate) manifest: Manifest,
    pub(crate) wasm: Vec<u8>,
    pub(crate) sha256: Sha256,
    pub(crate) signed: bool,
}

impl Package {
    pub(crate) fn new(manifest: Manifest, wasm: Vec<u8>, signed: bool) -> Self {
        let sha256 = Sha256::of(&wasm);

        Self {
            manifest,
            wasm,
            sha256,
            signed,
        }
    }
}
