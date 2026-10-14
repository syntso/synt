use crate::document::{DocumentPath, Revision};
use crate::merge::TomlText;
use crate::provider::SealedRevision;
use crate::seal::Keyring;

pub fn merge(base: Option<&[u8]>, older: &[u8], newer: &[u8]) -> (Option<Vec<u8>>, Vec<String>) {
    let merged = TomlText::merge(base, older, newer);

    (merged.body, merged.conflicts)
}

pub fn document_kind(text: &str) -> Option<String> {
    let path = DocumentPath::parse(text).ok()?;

    Some(path.kind().to_string())
}

pub fn open_sealed(json: &[u8]) -> bool {
    let Ok(sealed) = serde_json::from_slice::<SealedRevision>(json) else {
        return false;
    };
    let Ok(keyring) = Keyring::generate() else {
        return false;
    };

    Revision::open(&sealed.project, &keyring, &sealed).is_ok()
}
