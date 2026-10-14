use zeroize::Zeroizing;

pub trait SecretSource: Send + Sync + 'static {
    fn reveal(&self, name: &str, extension: &str) -> Result<Option<Zeroizing<Vec<u8>>>, String>;
}
