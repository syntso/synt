use std::fmt;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DeviceKey([u8; 32]);

impl DeviceKey {
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes)?;

        Ok(Self(bytes))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DeviceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeviceKey(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceKey;

    #[test]
    fn debug_output_hides_the_key() {
        let key = DeviceKey::from_bytes([7; 32]);
        assert_eq!(format!("{key:?}"), "DeviceKey(..)");
    }

    #[test]
    fn generated_keys_differ() {
        let one = DeviceKey::generate().unwrap();
        let other = DeviceKey::generate().unwrap();

        assert_ne!(one.expose(), other.expose());
    }
}
