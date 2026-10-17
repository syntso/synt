use std::fmt;
use std::path::PathBuf;

const USER_READABLE: &str = "any program running as this user can read and change the journal key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPlace {
    ProtectedKeychain,
    LoginKeychain,
    CredentialManager,
    Files(PathBuf),
}

impl KeyPlace {
    pub fn weakness(&self) -> Option<&'static str> {
        match self {
            Self::ProtectedKeychain => None,
            Self::LoginKeychain => Some(
                "this build is not signed with a keychain access group, so the data protection \
                 keychain is unavailable; other programs of this user may be able to read and \
                 change the journal key",
            ),
            Self::CredentialManager | Self::Files(_) => Some(USER_READABLE),
        }
    }
}

impl fmt::Display for KeyPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedKeychain => f.write_str("the data protection keychain"),
            Self::LoginKeychain => f.write_str("the login keychain"),
            Self::CredentialManager => f.write_str("Windows Credential Manager"),
            Self::Files(directory) => write!(f, "plain files in {}", directory.display()),
        }
    }
}
