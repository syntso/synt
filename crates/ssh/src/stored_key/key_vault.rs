use zeroize::Zeroizing;

pub trait KeyVault: Send + Sync {
    fn private_key(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, String>;
}
