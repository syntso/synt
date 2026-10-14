use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::{DeviceKey, JournalId};
use crate::JournalError;

const INFO: &[u8] = b"synt-journal/v1";

#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct JournalKey([u8; 32]);

impl JournalKey {
    pub(crate) fn derive(device: &DeviceKey, journal: JournalId) -> Result<Self, JournalError> {
        let hkdf = Hkdf::<Sha256>::new(Some(journal.as_bytes()), device.expose());

        let mut key = [0; 32];
        hkdf.expand(INFO, &mut key)?;

        Ok(Self(key))
    }

    pub(crate) fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::JournalKey;
    use crate::chain::{DeviceKey, JournalId};

    #[test]
    fn each_journal_gets_its_own_key() {
        let device = DeviceKey::from_bytes([1; 32]);

        let one = JournalKey::derive(&device, JournalId::from_bytes([1; 16])).unwrap();
        let other = JournalKey::derive(&device, JournalId::from_bytes([2; 16])).unwrap();

        assert_ne!(one.expose(), other.expose());
    }

    #[test]
    fn derivation_is_stable() {
        let device = DeviceKey::from_bytes([1; 32]);
        let journal = JournalId::from_bytes([1; 16]);

        assert_eq!(
            JournalKey::derive(&device, journal).unwrap().expose(),
            JournalKey::derive(&device, journal).unwrap().expose()
        );
    }
}
