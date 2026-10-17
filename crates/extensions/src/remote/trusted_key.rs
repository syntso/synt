use minisign_verify::{PublicKey, Signature};
use synt_journal::Timestamp;

use super::Listing;
use super::seal::{IndexSeal, ListingSeal};
use crate::ExtensionError;
use crate::sha256::Sha256;

const MILLIS_PER_SECOND: i64 = 1000;

#[derive(Debug, Clone)]
pub(crate) struct TrustedKey {
    key: PublicKey,
}

impl TrustedKey {
    pub(crate) fn from_base64(key: &str) -> Result<Self, ExtensionError> {
        let key = PublicKey::from_base64(key).map_err(ExtensionError::PublicKey)?;

        Ok(Self { key })
    }

    pub(crate) fn decode(text: &str) -> Result<Self, ExtensionError> {
        let key = PublicKey::decode(text).map_err(ExtensionError::PublicKey)?;

        Ok(Self { key })
    }

    pub(crate) fn check_index(
        &self,
        index: &[u8],
        signature: &[u8],
        now: Timestamp,
    ) -> Result<IndexSeal, ExtensionError> {
        let signature = self.verify(index, signature)?;
        let seal: IndexSeal = serde_json::from_str(signature.trusted_comment())
            .map_err(ExtensionError::IndexComment)?;

        let now_seconds = u64::try_from(now.as_millis() / MILLIS_PER_SECOND).unwrap_or_default();

        if seal.expired_at(now_seconds) {
            return Err(ExtensionError::IndexExpired);
        }

        Ok(seal)
    }

    pub(crate) fn check(&self, wasm: &[u8], listing: &Listing) -> Result<(), ExtensionError> {
        let found = Sha256::of(wasm);
        if found != *listing.sha256() {
            return Err(ExtensionError::DownloadDigest {
                found,
                listed: listing.sha256().clone(),
            });
        }

        let signature = self.verify(wasm, listing.signature().as_bytes())?;
        let sealed: ListingSeal = serde_json::from_str(signature.trusted_comment())
            .map_err(ExtensionError::ListingComment)?;
        if !sealed.covers(listing) {
            return Err(ExtensionError::ListingDiffers);
        }

        Ok(())
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<Signature, ExtensionError> {
        let text = std::str::from_utf8(signature).map_err(ExtensionError::SignatureText)?;
        let signature = Signature::decode(text)?;

        self.key.verify(data, &signature, false)?;

        Ok(signature)
    }
}
