use std::io::Cursor;

use minisign::KeyPair;
use serde_json::{Value, json};

pub struct Signer {
    pair: KeyPair,
}

impl Signer {
    pub fn new() -> Self {
        Self {
            pair: KeyPair::generate_unencrypted_keypair().unwrap(),
        }
    }

    pub fn public_key(&self) -> String {
        self.pair.pk.to_base64()
    }

    pub fn sign(&self, bytes: &[u8], sealed: &Value) -> String {
        let trusted = sealed.to_string();
        let signature = minisign::sign(
            Some(&self.pair.pk),
            &self.pair.sk,
            Cursor::new(bytes),
            Some(&trusted),
            Some("signed by the test registry"),
        )
        .unwrap();

        signature.to_string()
    }

    pub fn listing(&self, id: &str, file: &str, bytes: &[u8], capabilities: &Value) -> Value {
        self.listing_at("0.1.0", id, file, bytes, capabilities)
    }

    pub fn listing_at(
        &self,
        version: &str,
        id: &str,
        file: &str,
        bytes: &[u8],
        capabilities: &Value,
    ) -> Value {
        let sha256 = sha256(bytes);
        let sealed = json!({
            "id": id,
            "name": "Test",
            "version": version,
            "description": "Exercises the extension host.",
            "repository": "https://example.com/t",
            "sha256": sha256,
            "capabilities": capabilities,
        });

        json!({
            "id": id,
            "name": "Test",
            "version": version,
            "api": "0.1",
            "description": "Exercises the extension host.",
            "repository": "https://example.com/t",
            "file": file,
            "sha256": sha256,
            "signature": self.sign(bytes, &sealed),
            "capabilities": capabilities,
        })
    }

    pub fn sign_index(&self, index: &[u8], serial: u64, expires: u64) -> String {
        self.sign(index, &json!({ "serial": serial, "expires": expires }))
    }
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    data_encoding::HEXLOWER.encode(&Sha256::digest(bytes))
}
