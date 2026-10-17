pub trait VaultKind: Send + Sync + 'static {
    const NOUN: &'static str;
    const VAULT_PREFIX: &'static str;
}
