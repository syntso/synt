mod device_key;
mod fields;
mod key;
mod link;

use hmac::digest::Key;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use self::key::JournalKey;
use crate::error::JournalError;
use crate::journal::JournalId;

pub use device_key::DeviceKey;
pub(crate) use fields::Fields;
pub use link::Link;

const GENESIS: &[u8] = b"genesis";

pub(crate) struct Chain {
    mac: Hmac<Sha256>,
}

impl Chain {
    pub(crate) fn derive(device: &DeviceKey, journal: JournalId) -> Result<Self, JournalError> {
        let key = JournalKey::derive(device, journal)?;

        let mut block = Key::<Hmac<Sha256>>::default();
        block[..key.expose().len()].copy_from_slice(key.expose());

        Ok(Self {
            mac: Hmac::new(&block),
        })
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
    use super::{Chain, DeviceKey};
    use crate::journal::JournalId;

    fn chain(key: u8) -> Chain {
        let device = DeviceKey::from_bytes([key; 32]);
        Chain::derive(&device, JournalId::from_bytes([0; 16])).unwrap()
    }

    #[test]
    fn links_depend_on_the_previous_link() {
        let chain = chain(1);
        let genesis = chain.genesis();
        let first = chain.link(&genesis, b"event");

        assert_ne!(chain.link(&genesis, b"event"), chain.link(&first, b"event"));
    }

    #[test]
    fn links_depend_on_the_key() {
        let genesis = chain(1).genesis();

        assert_ne!(
            chain(1).link(&genesis, b"event"),
            chain(2).link(&genesis, b"event")
        );
    }
}
