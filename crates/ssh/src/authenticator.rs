mod agent;
mod candidates;
mod credentials;
mod key;
mod logins;

use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use russh::AgentAuthError;
use russh::client::{Handle, Handler};
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use tokio::task;

use agent::Agent;
use candidates::Candidates;
pub(crate) use credentials::Credentials;
pub(crate) use key::KeyText;
use key::{Candidate, KeyFile};
pub(crate) use logins::Logins;

use crate::{KeyVault, SshError, StoredKey, Target};

const SECRET_PREFIX: &str = "secret://";

pub(crate) struct Authenticator<'a> {
    target: &'a Target,
    vault: Option<Arc<dyn KeyVault>>,
    agent: Option<Agent>,
    candidates: Candidates,
    tried: Vec<String>,
    encrypted: Option<PathBuf>,
}

impl<'a> Authenticator<'a> {
    pub(crate) fn new(
        target: &'a Target,
        credentials: Credentials,
        vault: Option<Arc<dyn KeyVault>>,
    ) -> Self {
        let Credentials { agent, candidates } = credentials;

        Self {
            target,
            vault,
            agent,
            candidates,
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
        match candidate {
            Candidate::Agent(key) => self.try_agent_key(handle, key).await,
            Candidate::File { path, .. } => self.try_key_file(handle, &path).await,
            Candidate::Vault(name) => self.try_vault_key(handle, &name).await,
        }
    }

    async fn try_agent_key<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        key: PublicKey,
    ) -> Result<Option<PublicKey>, SshError> {
        let hash = Self::hash_for(handle, &key).await;

        let Some(agent) = self.agent.as_mut() else {
            return Ok(None);
        };

        self.tried
            .push(format!("ssh-agent {}", key.fingerprint(HashAlg::Sha256)));

        let result = handle
            .authenticate_publickey_with(self.target.user(), key.clone(), hash, agent.signer())
            .await;

        match result {
            Ok(result) => Ok(result.success().then_some(key)),
            Err(AgentAuthError::Key(_)) => Ok(None),
            Err(AgentAuthError::Send(error)) => Err(self.lost(error)),
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

        self.offer(handle, *key, path.display().to_string()).await
    }

    async fn try_vault_key<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        name: &str,
    ) -> Result<Option<PublicKey>, SshError> {
        let key = Self::vault_key(self.vault.clone(), name).await?;

        self.offer(handle, key, format!("{SECRET_PREFIX}{name}"))
            .await
    }

    async fn offer<H: Handler>(
        &mut self,
        handle: &mut Handle<H>,
        key: PrivateKey,
        label: String,
    ) -> Result<Option<PublicKey>, SshError> {
        let public = key.public_key().clone();
        let hash = Self::hash_for(handle, &public).await;
        let signing = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
        self.tried.push(label);

        let result = handle
            .authenticate_publickey(self.target.user(), signing)
            .await
            .map_err(|error| self.lost(error))?;

        Ok(result.success().then_some(public))
    }

    async fn vault_key(
        vault: Option<Arc<dyn KeyVault>>,
        name: &str,
    ) -> Result<PrivateKey, SshError> {
        let Some(vault) = vault else {
            return Err(SshError::NoKeyVault(name.to_owned()));
        };

        let owned = name.to_owned();
        let reading = task::spawn_blocking(move || vault.private_key(&owned)).await;

        let bytes = match reading {
            Ok(Ok(Some(bytes))) => bytes,
            Ok(Ok(None)) => return Err(SshError::NotInVault(name.to_owned())),
            Ok(Err(cause)) => {
                return Err(SshError::VaultRead {
                    name: name.to_owned(),
                    cause: Arc::new(cause),
                });
            }
            Err(_) => return Err(SshError::VaultStopped(name.to_owned())),
        };

        StoredKey::decode(&bytes).ok_or_else(|| SshError::DamagedVaultKey(name.to_owned()))
    }

    fn failure(self) -> SshError {
        match self.encrypted {
            Some(path) => SshError::EncryptedKey { path },
            None => SshError::NoAuthentication { tried: self.tried },
        }
    }

    fn lost(&self, error: impl std::error::Error + Send + Sync + 'static) -> SshError {
        SshError::connect(self.target.address(), error)
    }

    async fn hash_for<H: Handler>(handle: &Handle<H>, key: &PublicKey) -> Option<HashAlg> {
        if !key.algorithm().is_rsa() {
            return None;
        }

        match handle.best_supported_rsa_hash().await {
            Ok(Some(Some(hash))) => Some(hash),
            _ => Some(HashAlg::Sha256),
        }
    }
}
