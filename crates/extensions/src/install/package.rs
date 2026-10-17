use crate::manifest::Manifest;
use crate::sha256::Sha256;

pub(super) const MANIFEST_FILE: &str = "extension.toml";
pub(super) const COMPONENT_FILE: &str = "extension.wasm";

#[derive(Debug, Clone)]
pub(crate) struct Package {
    manifest: Manifest,
    wasm: Vec<u8>,
    sha256: Sha256,
    signed: bool,
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

    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub(crate) fn wasm(&self) -> &[u8] {
        &self.wasm
    }

    pub(crate) fn sha256(&self) -> &Sha256 {
        &self.sha256
    }

    pub(crate) fn signed(&self) -> bool {
        self.signed
    }
}
