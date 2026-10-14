use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum KeySource {
    Agent,
    File(PathBuf),
    Vault(String),
}
