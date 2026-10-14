use minisign_verify::{PublicKey, Signature};

use super::sealed::Sealed;
use super::{IndexSeal, Listing};
use crate::ExtensionError;
use crate::package::sha256;
use crate::shelf::now;

#[derive(Debug, Clone)]
pub(crate) struct TrustedKey {
    key: PublicKey,
}

impl TrustedKey {
    pub(crate) fn from_base64(key: &str) -> Result<Self, ExtensionError> {
        let key = PublicKey::from_base64(key)
            .map_err(|error| ExtensionError::Signature(format!("the public key: {error}")))?;

        Ok(Self { key })
    }

    pub(crate) fn decode(text: &str) -> Result<Self, ExtensionError> {
        let key = PublicKey::decode(text)
            .map_err(|error| ExtensionError::Signature(format!("the public key: {error}")))?;

        Ok(Self { key })
    }

    pub(crate) fn check_index(
        &self,
        index: &[u8],
        signature: &[u8],
    ) -> Result<IndexSeal, ExtensionError> {
        let signature = self.verify(index, signature)?;
        let seal: IndexSeal =
            serde_json::from_str(signature.trusted_comment()).map_err(|error| {
                ExtensionError::Signature(format!("the trusted comment of the index: {error}"))
            })?;

        if seal.expires <= now() / 1000 {
            return Err(ExtensionError::Signature(
                "the registry index has expired; the registry has not been signed lately"
                    .to_owned(),
            ));
        }

        Ok(seal)
    }

    pub(crate) fn check(&self, wasm: &[u8], listing: &Listing) -> Result<(), ExtensionError> {
        let found = sha256(wasm);
        if found != listing.sha256 {
            return Err(ExtensionError::Signature(format!(
                "the download has sha256 {found}, the index says {}",
                listing.sha256
            )));
        }

        let signature = self.verify(wasm, listing.signature.as_bytes())?;
        let sealed: Sealed = serde_json::from_str(signature.trusted_comment())
            .map_err(|error| ExtensionError::Signature(format!("the trusted comment: {error}")))?;
        if !sealed.covers(listing) {
            return Err(ExtensionError::Signature(
                "the signed id, name, version, description, repository, sha256 or capabilities \
                 differ from the index"
                    .to_owned(),
            ));
        }

        Ok(())
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<Signature, ExtensionError> {
        let text = std::str::from_utf8(signature)
            .map_err(|error| ExtensionError::Signature(error.to_string()))?;
        let signature = Signature::decode(text)
            .map_err(|error| ExtensionError::Signature(error.to_string()))?;

        self.key
            .verify(data, &signature, false)
            .map_err(|error| ExtensionError::Signature(error.to_string()))?;

        Ok(signature)
    }
}
