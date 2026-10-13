use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;

use russh::AgentAuthError;
use russh::client::{AuthResult, Handle, Handler};
use russh::keys::agent::AgentIdentity;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKey};

use super::{Agent, KeyFile, agent};
use crate::home::ssh_directory;
use crate::{SshError, Target};

const DEFAULT_KEYS: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];
const AGENT_ATTEMPTS: usize = 5;

pub(crate) struct Authenticator<'a> {
    target: &'a Target,
    tried: Vec<String>,
    encrypted: Option<PathBuf>,
}

impl<'a> Authenticator<'a> {
    pub(crate) fn new(target: &'a Target) -> Self {
        Self {
            target,
            tried: Vec::new(),
            encrypted: None,
        }
    }

    pub(crate) async fn authenticate<H: Handler>(
        mut self,
        handle: &mut Handle<H>,
    ) -> Result<(), SshError> {
        let pinned = self.pinned_keys().await;

        if self.try_agent(handle, pinned.as_deref()).await? {
            return Ok(());
        }

        for path in self.key_paths() {
            if self.try_key_file(handle, path).await? {
                return Ok(());
            }
        }

        Err(self.failure())
    }

    async fn pinned_keys(&self) -> Option<Vec<PublicKey>> {
        let files = self.target.identity_files();

        if files.is_empty() {
            return None;
        }

        let mut keys = Vec::new();

        for path in files {
            if let Some(key) = KeyFile::public_key(path).await {
                keys.push(key);
            }
        }

        Some(keys)
    }

    async fn try_agent<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        pinned: Option<&[PublicKey]>,
    ) -> Result<bool, SshError> {
        let Some(mut agent) = agent::connect().await else {
            return Ok(false);
        };
        let Ok(identities) = agent.request_identities().await else {
            return Ok(false);
        };

        let offered: Vec<PublicKey> = identities
            .into_iter()
            .filter_map(public_key)
            .filter(|key| is_wanted(pinned, key))
            .take(AGENT_ATTEMPTS)
            .collect();

        for key in offered {
            if self.try_agent_key(handle, &mut agent, key).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn try_agent_key<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        agent: &mut Agent,
        key: PublicKey,
    ) -> Result<bool, SshError> {
        let hash = rsa_hash(handle, &key).await;
        self.tried
            .push(format!("ssh-agent {}", key.fingerprint(HashAlg::Sha256)));

        let result = handle
            .authenticate_publickey_with(self.target.user(), key, hash, agent)
            .await;

        match result {
            Ok(result) => Ok(accepted(&result)),
            Err(AgentAuthError::Key(_)) => Ok(false),
            Err(AgentAuthError::Send(error)) => Err(self.lost(&error)),
        }
    }

    async fn try_key_file<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        path: PathBuf,
    ) -> Result<bool, SshError> {
        let key = match KeyFile::read(&path).await? {
            KeyFile::Missing => return Ok(false),
            KeyFile::Encrypted => {
                self.encrypted.get_or_insert(path);
                return Ok(false);
            }
            KeyFile::Unreadable => {
                self.tried.push(format!("{} (unreadable)", path.display()));
                return Ok(false);
            }
            KeyFile::Unsupported => {
                self.tried.push(format!(
                    "{} (RSA works through ssh-agent only)",
                    path.display()
                ));
                return Ok(false);
            }
            KeyFile::Loaded(key) => key,
        };

        let hash = rsa_hash(handle, key.public_key()).await;
        let signing = PrivateKeyWithHashAlg::new(Arc::new(*key), hash);
        self.tried.push(path.display().to_string());

        let result = handle
            .authenticate_publickey(self.target.user(), signing)
            .await
            .map_err(|error| self.lost(&error))?;

        Ok(accepted(&result))
    }

    fn key_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.target.identity_files().to_vec();

        let defaults = ssh_directory()
            .map(|directory| DEFAULT_KEYS.map(|name| directory.join(name)).to_vec())
            .unwrap_or_default();

        for path in defaults {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }

        paths
    }

    fn failure(self) -> SshError {
        match self.encrypted {
            Some(path) => SshError::EncryptedKey { path },
            None => SshError::NoAuthentication { tried: self.tried },
        }
    }

    fn lost(&self, error: &impl Display) -> SshError {
        SshError::Connect {
            host: self.target.address(),
            reason: error.to_string(),
        }
    }
}

async fn rsa_hash<H: Handler>(handle: &Handle<H>, key: &PublicKey) -> Option<HashAlg> {
    if !key.algorithm().is_rsa() {
        return None;
    }

    match handle.best_supported_rsa_hash().await {
        Ok(Some(hash)) => hash,
        _ => Some(HashAlg::Sha256),
    }
}

fn public_key(identity: AgentIdentity) -> Option<PublicKey> {
    match identity {
        AgentIdentity::PublicKey { key, .. } => Some(key),
        AgentIdentity::Certificate { .. } => None,
    }
}

fn is_wanted(pinned: Option<&[PublicKey]>, key: &PublicKey) -> bool {
    let Some(pinned) = pinned else {
        return true;
    };

    pinned
        .iter()
        .any(|known| known.key_data() == key.key_data())
}

fn accepted(result: &AuthResult) -> bool {
    matches!(result, AuthResult::Success)
}
