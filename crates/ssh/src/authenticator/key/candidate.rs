use russh::keys::PublicKey;

use super::KeySource;

#[derive(Debug, Clone)]
pub(super) struct Candidate {
    pub(super) key: Option<PublicKey>,
    pub(super) source: KeySource,
}

impl Candidate {
    pub(super) fn holds(&self, key: &PublicKey) -> bool {
        self.key
            .as_ref()
            .is_some_and(|own| own.key_data() == key.key_data())
    }
}
