#[cfg(target_os = "macos")]
pub const WEAK_VAULT_FLAG: &str = "--allow-login-keychain";

#[cfg(not(target_os = "macos"))]
pub const WEAK_VAULT_FLAG: &str = "--allow-user-readable-keys";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeakVault {
    Refuse,
    Allow,
}

impl WeakVault {
    pub fn allowed(self) -> bool {
        self == Self::Allow || cfg!(debug_assertions)
    }
}
