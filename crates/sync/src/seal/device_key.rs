use std::fmt;

use synt_platform::Vault;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret};
use zeroize::Zeroizing;

use super::SecretKey;
use crate::SyncError;
use crate::encoding::PublicKey;
use crate::id::MemberId;
use crate::provider::MemberKey;
use crate::random::random_bytes;

const VAULT_ITEM: &str = "sync.device";

pub(crate) struct DeviceKey {
    secret: StaticSecret,
}

impl DeviceKey {
    pub(crate) fn generate() -> Result<Self, SyncError> {
        let bytes = Zeroizing::new(random_bytes::<{ SecretKey::BYTES }>()?);

        Ok(Self {
            secret: StaticSecret::from(*bytes),
        })
    }

    pub(crate) fn load_or_create(vault: &dyn Vault) -> Result<Self, SyncError> {
        if let Some(bytes) = vault.read(VAULT_ITEM)? {
            return Self::from_slice(&bytes);
        }

        let device = Self::generate()?;
        vault.write(VAULT_ITEM, device.secret.as_bytes())?;

        Ok(device)
    }

    pub(crate) fn public_key(&self) -> PublicKey {
        PublicKey::new(X25519Public::from(&self.secret).to_bytes())
    }

    pub(crate) fn member_key(&self) -> MemberKey {
        MemberKey::new(self.public_key())
    }

    pub(crate) fn member(&self) -> MemberId {
        self.member_key().member
    }

    pub(crate) fn agree(
        &self,
        public_key: &PublicKey,
    ) -> Result<Zeroizing<[u8; SecretKey::BYTES]>, SyncError> {
        let shared = self
            .secret
            .diffie_hellman(&X25519Public::from(*public_key.get()));

        if !shared.was_contributory() {
            return Err(SyncError::Crypto("the other public key is not usable"));
        }

        Ok(Zeroizing::new(shared.to_bytes()))
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, SyncError> {
        let bytes: [u8; SecretKey::BYTES] = bytes
            .try_into()
            .map_err(|_| SyncError::Crypto("the device key in the vault is damaged"))?;
        let bytes = Zeroizing::new(bytes);

        Ok(Self {
            secret: StaticSecret::from(*bytes),
        })
    }
}

impl fmt::Debug for DeviceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceKey")
            .field("member", &self.member())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceKey;
    use crate::encoding::PublicKey;

    #[test]
    fn two_devices_agree_on_the_same_secret() {
        let one = DeviceKey::generate().unwrap();
        let other = DeviceKey::generate().unwrap();

        let ours = one.agree(&other.public_key()).unwrap();
        let theirs = other.agree(&one.public_key()).unwrap();

        assert_eq!(ours, theirs);
    }

    #[test]
    fn a_low_order_public_key_is_refused() {
        let device = DeviceKey::generate().unwrap();

        assert!(device.agree(&PublicKey::new([0; 32])).is_err());
    }
}
