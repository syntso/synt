use std::fmt::Display;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use russh::AgentAuthError;
use russh::client::{AuthResult, Handle, Handler};
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use tokio::task;

use super::{Agent, Candidate, Credentials, KeyFile, KeySource};
use crate::stored_key::decode_private_key;
use crate::{KeyVault, SshError, Target};

const SECRET_PREFIX: &str = "secret://";

pub(crate) struct Authenticator<'a> {
    target: &'a Target,
    vault: Option<Arc<dyn KeyVault>>,
    agent: Option<Agent>,
    candidates: Vec<Candidate>,
    tried: Vec<String>,
    encrypted: Option<PathBuf>,
}

impl<'a> Authenticator<'a> {
    pub(crate) fn new(
        target: &'a Target,
        credentials: Credentials,
        vault: Option<Arc<dyn KeyVault>>,
    ) -> Self {
        Self {
            target,
            vault,
            agent: credentials.agent,
            candidates: credentials.candidates,
            tried: Vec::new(),
            encrypted: None,
        }
    }

    pub(crate) async fn authenticate<H: Handler>(
        mut self,
        handle: &mut Handle<H>,
    ) -> Result<PublicKey, SshError> {
        let candidates = mem::take(&mut self.candidates);

        for candidate in candidates {
            if let Some(accepted) = self.try_candidate(handle, candidate).await? {
                return Ok(accepted);
            }
        }

        Err(self.failure())
    }

    async fn try_candidate<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        candidate: Candidate,
    ) -> Result<Option<PublicKey>, SshError> {
        match (candidate.source, candidate.key) {
            (KeySource::Agent, Some(key)) => self.try_agent_key(handle, key).await,
            (KeySource::File(path), _) => self.try_key_file(handle, &path).await,
            (KeySource::Vault(name), _) => self.try_vault_key(handle, &name).await,
            (KeySource::Agent, None) => Ok(None),
        }
    }

    async fn try_agent_key<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        key: PublicKey,
    ) -> Result<Option<PublicKey>, SshError> {
        let hash = rsa_hash(handle, &key).await;

        let Some(agent) = self.agent.as_mut() else {
            return Ok(None);
        };

        self.tried
            .push(format!("ssh-agent {}", key.fingerprint(HashAlg::Sha256)));

        let result = handle
            .authenticate_publickey_with(self.target.user(), key.clone(), hash, agent)
            .await;

        match result {
            Ok(result) => Ok(accepted(&result).then_some(key)),
            Err(AgentAuthError::Key(_)) => Ok(None),
            Err(AgentAuthError::Send(error)) => Err(self.lost(&error)),
        }
    }

    async fn try_key_file<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        path: &Path,
    ) -> Result<Option<PublicKey>, SshError> {
        let key = match KeyFile::read(path).await? {
            KeyFile::Missing => return Ok(None),
            KeyFile::Encrypted => {
                self.encrypted.get_or_insert_with(|| path.to_path_buf());
                return Ok(None);
            }
            KeyFile::Unreadable => {
                self.tried.push(format!("{} (unreadable)", path.display()));
                return Ok(None);
            }
            KeyFile::Unsupported => {
                self.tried.push(format!(
                    "{} (RSA works through ssh-agent only)",
                    path.display()
                ));
                return Ok(None);
            }
            KeyFile::Loaded(key) => key,
        };

        let public = key.public_key().clone();
        let hash = rsa_hash(handle, &public).await;
        let signing = PrivateKeyWithHashAlg::new(Arc::new(*key), hash);
        self.tried.push(path.display().to_string());

        let result = handle
            .authenticate_publickey(self.target.user(), signing)
            .await
            .map_err(|error| self.lost(&error))?;

        Ok(accepted(&result).then_some(public))
    }

    async fn try_vault_key<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        name: &str,
    ) -> Result<Option<PublicKey>, SshError> {
        let key = read_vault_key(self.vault.clone(), name).await?;
        let public = key.public_key().clone();
        let hash = rsa_hash(handle, &public).await;
        let signing = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
        self.tried.push(format!("{SECRET_PREFIX}{name}"));

        let result = handle
            .authenticate_publickey(self.target.user(), signing)
            .await
            .map_err(|error| self.lost(&error))?;

        Ok(accepted(&result).then_some(public))
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
        Ok(Some(Some(hash))) => Some(hash),
        _ => Some(HashAlg::Sha256),
    }
}

async fn read_vault_key(
    vault: Option<Arc<dyn KeyVault>>,
    name: &str,
) -> Result<PrivateKey, SshError> {
    let Some(vault) = vault else {
        return Err(vault_error(name, "this core has no key vault"));
    };

    let owned = name.to_owned();
    let reading = task::spawn_blocking(move || vault.private_key(&owned));

    let bytes = match reading.await {
        Ok(Ok(Some(bytes))) => bytes,
        Ok(Ok(None)) => return Err(vault_error(name, "there is no such key in the vault")),
        Ok(Err(reason)) => return Err(vault_error(name, &reason)),
        Err(_) => return Err(vault_error(name, "reading the vault stopped unexpectedly")),
    };

    decode_private_key(&bytes).ok_or_else(|| vault_error(name, "the key in the vault is damaged"))
}

fn vault_error(name: &str, reason: &str) -> SshError {
    SshError::VaultKey {
        name: name.to_owned(),
        reason: reason.to_owned(),
    }
}

fn accepted(result: &AuthResult) -> bool {
    matches!(result, AuthResult::Success)
}
