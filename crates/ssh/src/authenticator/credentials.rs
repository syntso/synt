use std::path::PathBuf;

use russh::keys::PublicKey;

use super::{Agent, Candidates};
use crate::Target;
use crate::directory::SshDirectory;

const DEFAULT_KEYS: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];
const MOST_ATTEMPTS: usize = 5;

pub(crate) struct Credentials {
    pub(super) agent: Option<Agent>,
    pub(super) candidates: Candidates,
}

impl Credentials {
    pub(crate) async fn gather(
        target: &Target,
        accepted_before: Option<PublicKey>,
        directory: &SshDirectory,
    ) -> Self {
        if let Some(name) = target.vault_key() {
            return Self::vault_only(name);
        }

        let mut agent = Agent::connect().await;
        let held = match agent.as_mut() {
            Some(agent) => agent.held_keys().await,
            None => Vec::new(),
        };

        let paths = Self::key_paths(target, directory);
        let mut candidates = Candidates::from_files(&paths, &held).await;

        if target.identity_files().is_empty() {
            candidates.add_agent_keys(held);
        }

        if let Some(key) = accepted_before {
            candidates.put_first(&key);
        }

        candidates.truncate(MOST_ATTEMPTS);

        Self { agent, candidates }
    }

    fn vault_only(name: &str) -> Self {
        Self {
            agent: None,
            candidates: Candidates::vault_only(name),
        }
    }

    fn key_paths(target: &Target, directory: &SshDirectory) -> Vec<PathBuf> {
        let configured = target.identity_files();

        if !configured.is_empty() {
            return configured.to_vec();
        }

        directory
            .path()
            .map(|directory| DEFAULT_KEYS.map(|name| directory.join(name)).to_vec())
            .unwrap_or_default()
    }
}
