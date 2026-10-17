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

    pub fn reason(self) -> String {
        match self {
            Self::Allow => format!("allowed by {WEAK_VAULT_FLAG}, which is for development only"),
            Self::Refuse => format!(
                "allowed only because this is a debug build; a release build refuses this \
                 without {WEAK_VAULT_FLAG}"
            ),
        }
    }
}
