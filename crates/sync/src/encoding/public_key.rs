use std::fmt;

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Base64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey([u8; PublicKey::BYTES]);

impl PublicKey {
    pub(crate) const BYTES: usize = 32;

    pub fn new(bytes: [u8; Self::BYTES]) -> Self {
        Self(bytes)
    }

    pub fn get(&self) -> &[u8; Self::BYTES] {
        &self.0
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = serde_json::to_string(self).map_err(|_| fmt::Error)?;

        write!(formatter, "PublicKey({text})")
    }
}

impl Serialize for PublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Base64::new(self.0.to_vec()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = Base64::deserialize(deserializer)?.into_bytes();

        let bytes: [u8; Self::BYTES] = bytes
            .try_into()
            .map_err(|_| D::Error::custom(format!("a public key has {} bytes", Self::BYTES)))?;

        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::PublicKey;

    #[test]
    fn a_key_of_the_wrong_length_is_refused() {
        assert!(serde_json::from_str::<PublicKey>("\"c3ludA==\"").is_err());
    }

    #[test]
    fn a_key_round_trips() {
        let key = PublicKey::new([7; 32]);

        let text = serde_json::to_string(&key).unwrap();

        assert_eq!(serde_json::from_str::<PublicKey>(&text).unwrap(), key);
    }
}
