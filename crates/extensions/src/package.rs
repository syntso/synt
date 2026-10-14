use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};

use crate::manifest::Manifest;

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

pub(crate) fn sha256(bytes: &[u8]) -> String {
    HEXLOWER.encode(&Sha256::digest(bytes))
}
