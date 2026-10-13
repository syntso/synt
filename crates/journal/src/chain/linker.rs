use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use super::{JournalKey, Link};

const GENESIS: &[u8] = b"genesis";

pub(crate) struct Linker {
    mac: Hmac<Sha256>,
}

impl Linker {
    pub(crate) fn new(key: &JournalKey) -> Self {
        let mac =
            Hmac::<Sha256>::new_from_slice(key.expose()).expect("HMAC accepts keys of any length");
        Self { mac }
    }

    pub(crate) fn genesis(&self) -> Link {
        self.sign(&[GENESIS])
    }

    pub(crate) fn link(&self, previous: &Link, canonical: &[u8]) -> Link {
        self.sign(&[previous.as_bytes(), canonical])
    }

    fn sign(&self, parts: &[&[u8]]) -> Link {
        let mut mac = self.mac.clone();

        for part in parts {
            mac.update(part);
        }

        Link::from_bytes(mac.finalize().into_bytes().into())
    }
}

#[cfg(test)]
mod tests {
    use super::Linker;
    use crate::chain::{DeviceKey, JournalId, JournalKey};

    fn linker(key: u8) -> Linker {
        let device = DeviceKey::from_bytes([key; 32]);
        Linker::new(&JournalKey::derive(&device, JournalId::from_bytes([0; 16])))
    }

    #[test]
    fn links_depend_on_the_previous_link() {
        let linker = linker(1);
        let genesis = linker.genesis();
        let first = linker.link(&genesis, b"event");

        assert_ne!(
            linker.link(&genesis, b"event"),
            linker.link(&first, b"event")
        );
    }

    #[test]
    fn links_depend_on_the_key() {
        let genesis = linker(1).genesis();
        assert_ne!(
            linker(1).link(&genesis, b"event"),
            linker(2).link(&genesis, b"event")
        );
    }
}
