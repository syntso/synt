use std::fmt::Display;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use russh::AgentAuthError;
use russh::client::{AuthResult, Handle, Handler};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKey};

use super::{Agent, Candidate, Credentials, KeyFile, KeySource};
use crate::{SshError, Target};

pub(crate) struct Authenticator<'a> {
    target: &'a Target,
    agent: Option<Agent>,
    candidates: Vec<Candidate>,
    tried: Vec<String>,
    encrypted: Option<PathBuf>,
}

impl<'a> Authenticator<'a> {
    pub(crate) fn new(target: &'a Target, credentials: Credentials) -> Self {
        Self {
            target,
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

fn accepted(result: &AuthResult) -> bool {
    matches!(result, AuthResult::Success)
}
