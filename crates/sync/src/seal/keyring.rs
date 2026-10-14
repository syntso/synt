use std::collections::BTreeMap;

use zeroize::Zeroizing;

use super::secret_key::SECRET_KEY_BYTES;
use super::{Naming, SecretKey};
use crate::SyncError;
use crate::id::{Generation, ProjectId};

const FORMAT: u8 = 1;
const ENTRY_BYTES: usize = 4 + SECRET_KEY_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Keyring {
    keys: BTreeMap<Generation, SecretKey>,
}

impl Keyring {
    pub(crate) fn generate() -> Result<Self, SyncError> {
        let keys = BTreeMap::from([(Generation::FIRST, SecretKey::generate()?)]);

        Ok(Self { keys })
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, SyncError> {
        let damaged = || SyncError::Crypto("the project keyring is damaged");

        let (format, entries) = bytes.split_first().ok_or_else(damaged)?;
        if *format != FORMAT || entries.is_empty() || entries.len() % ENTRY_BYTES != 0 {
            return Err(damaged());
        }

        let mut keys = BTreeMap::new();

        for entry in entries.as_chunks::<ENTRY_BYTES>().0 {
            let (generation, key) = entry.split_at(4);
            let generation = u32::from_be_bytes(generation.try_into().map_err(|_| damaged())?);
            let key = SecretKey::from_slice(key).ok_or_else(damaged)?;

            keys.insert(Generation::new(generation), key);
        }

        Ok(Self { keys })
    }

    pub(crate) fn to_bytes(&self) -> Zeroizing<Vec<u8>> {
        let mut bytes = Zeroizing::new(Vec::with_capacity(1 + self.keys.len() * ENTRY_BYTES));
        bytes.push(FORMAT);

        for (generation, key) in &self.keys {
            bytes.extend_from_slice(&generation.get().to_be_bytes());
            bytes.extend_from_slice(key.get());
        }

        bytes
    }

    pub(crate) fn rotated(&self) -> Result<Self, SyncError> {
        let next = self
            .generation()
            .next()
            .ok_or(SyncError::Crypto("the project key cannot rotate any more"))?;

        let mut keys = self.keys.clone();
        keys.insert(next, SecretKey::generate()?);

        Ok(Self { keys })
    }

    pub(crate) fn generation(&self) -> Generation {
        self.keys
            .keys()
            .next_back()
            .copied()
            .unwrap_or(Generation::FIRST)
    }

    pub(crate) fn current(&self) -> Result<(Generation, &SecretKey), SyncError> {
        let generation = self.generation();
        let key = self.key(generation)?;

        Ok((generation, key))
    }

    pub(crate) fn key(&self, generation: Generation) -> Result<&SecretKey, SyncError> {
        self.keys
            .get(&generation)
            .ok_or(SyncError::Crypto("no key of that generation"))
    }

    pub(crate) fn holds(&self, generation: Generation) -> bool {
        self.keys.contains_key(&generation)
    }

    pub(crate) fn naming(&self, project: &ProjectId) -> Result<Naming, SyncError> {
        let first = self
            .keys
            .values()
            .next()
            .ok_or(SyncError::Crypto("the keyring is empty"))?;

        Ok(Naming::derive(first, project))
    }

    pub(crate) fn keeps(&self, older: &Self) -> bool {
        older
            .keys
            .iter()
            .all(|(generation, key)| self.keys.get(generation) == Some(key))
    }
}

#[cfg(test)]
mod tests {
    use super::Keyring;
    use crate::id::Generation;

    #[test]
    fn a_new_keyring_starts_at_the_first_generation() {
        let keyring = Keyring::generate().unwrap();

        assert_eq!(keyring.generation(), Generation::FIRST);
    }

    #[test]
    fn rotating_adds_a_generation_and_keeps_the_old_keys() {
        let keyring = Keyring::generate().unwrap();

        let rotated = keyring.rotated().unwrap();

        assert_eq!(rotated.generation(), Generation::new(2));
        assert!(rotated.keeps(&keyring));
        assert!(!keyring.keeps(&rotated));
        assert_ne!(
            rotated.key(Generation::new(2)).unwrap(),
            rotated.key(Generation::FIRST).unwrap()
        );
    }

    #[test]
    fn a_keyring_with_another_first_key_is_not_kept() {
        let one = Keyring::generate().unwrap();
        let other = Keyring::generate().unwrap().rotated().unwrap();

        assert!(!other.keeps(&one));
    }

    #[test]
    fn bytes_round_trip() {
        let keyring = Keyring::generate().unwrap().rotated().unwrap();

        let restored = Keyring::from_bytes(&keyring.to_bytes()).unwrap();

        assert_eq!(restored, keyring);
    }

    #[test]
    fn damaged_bytes_are_refused() {
        let bytes = Keyring::generate().unwrap().to_bytes();

        for damaged in [
            &[][..],
            &bytes[..1],
            &bytes[..bytes.len() - 1],
            &[2, 0, 0, 0, 1],
        ] {
            assert!(Keyring::from_bytes(damaged).is_err());
        }
    }
}
