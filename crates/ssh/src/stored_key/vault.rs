use std::error::Error;

use zeroize::Zeroizing;

pub type VaultError = Box<dyn Error + Send + Sync>;

pub trait KeyVault: Send + Sync {
    fn private_key(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, VaultError>;
}
