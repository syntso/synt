use zeroize::Zeroizing;

pub trait SecretSource: Send + Sync + 'static {
    fn reveal(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, String>;
}
