use std::path::{Path, PathBuf};

use russh::keys::PublicKey;

use super::KeyFile;

#[derive(Debug, Clone)]
pub(in crate::authenticator) enum Candidate {
    Agent(PublicKey),
    File {
        path: PathBuf,
        public: Option<PublicKey>,
    },
    Vault(String),
}

impl Candidate {
    pub(in crate::authenticator) async fn from_file(
        path: &Path,
        held: &[PublicKey],
    ) -> Option<Self> {
        let public = KeyFile::public_key(path).await;

        let in_agent = public
            .as_ref()
            .filter(|key| held.iter().any(|own| own.key_data() == key.key_data()));

        if let Some(key) = in_agent {
            return Some(Self::Agent(key.clone()));
        }

        if !KeyFile::exists(path).await {
            return None;
        }

        Some(Self::File {
            path: path.to_path_buf(),
            public,
        })
    }

    pub(in crate::authenticator) fn public_key(&self) -> Option<&PublicKey> {
        match self {
            Self::Agent(key) => Some(key),
            Self::File { public, .. } => public.as_ref(),
            Self::Vault(_) => None,
        }
    }

    pub(in crate::authenticator) fn holds(&self, key: &PublicKey) -> bool {
        self.public_key()
            .is_some_and(|own| own.key_data() == key.key_data())
    }
}
