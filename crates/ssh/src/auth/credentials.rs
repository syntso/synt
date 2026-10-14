use std::path::{Path, PathBuf};

use russh::keys::PublicKey;
use russh::keys::agent::AgentIdentity;

use super::{Agent, Candidate, KeyFile, KeySource, agent};
use crate::Target;
use crate::ssh_directory::SshDirectory;

const DEFAULT_KEYS: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];
const MOST_ATTEMPTS: usize = 5;

pub(crate) struct Credentials {
    pub(super) agent: Option<Agent>,
    pub(super) candidates: Vec<Candidate>,
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

        let mut agent = agent::connect().await;
        let held = match agent.as_mut() {
            Some(agent) => held_keys(agent).await,
            None => Vec::new(),
        };

        let configured = !target.identity_files().is_empty();
        let mut candidates = file_candidates(&key_paths(target, directory), &held).await;

        if !configured {
            add_other_agent_keys(&mut candidates, held);
        }

        if let Some(key) = accepted_before {
            put_first(&mut candidates, &key);
        }

        candidates.truncate(MOST_ATTEMPTS);

        Self { agent, candidates }
    }

    fn vault_only(name: &str) -> Self {
        let candidate = Candidate {
            key: None,
            source: KeySource::Vault(name.to_owned()),
        };

        Self {
            agent: None,
            candidates: vec![candidate],
        }
    }
}

async fn held_keys(agent: &mut Agent) -> Vec<PublicKey> {
    let Ok(identities) = agent.request_identities().await else {
        return Vec::new();
    };

    identities.into_iter().filter_map(public_key).collect()
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

async fn file_candidates(paths: &[PathBuf], held: &[PublicKey]) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();

    for path in paths {
        let Some(candidate) = file_candidate(path, held).await else {
            continue;
        };

        let duplicate = candidate
            .key
            .as_ref()
            .is_some_and(|key| candidates.iter().any(|known| known.holds(key)));

        if !duplicate {
            candidates.push(candidate);
        }
    }

    candidates
}

async fn file_candidate(path: &Path, held: &[PublicKey]) -> Option<Candidate> {
    let key = KeyFile::public_key(path).await;

    let in_agent = key
        .as_ref()
        .is_some_and(|key| held.iter().any(|own| own.key_data() == key.key_data()));

    if in_agent {
        return Some(Candidate {
            key,
            source: KeySource::Agent,
        });
    }

    if !KeyFile::exists(path).await {
        return None;
    }

    Some(Candidate {
        key,
        source: KeySource::File(path.to_path_buf()),
    })
}

fn add_other_agent_keys(candidates: &mut Vec<Candidate>, held: Vec<PublicKey>) {
    for key in held {
        if candidates.iter().any(|known| known.holds(&key)) {
            continue;
        }

        candidates.push(Candidate {
            key: Some(key),
            source: KeySource::Agent,
        });
    }
}

fn put_first(candidates: &mut [Candidate], key: &PublicKey) {
    if let Some(position) = candidates.iter().position(|known| known.holds(key)) {
        candidates[..=position].rotate_right(1);
    }
}

fn public_key(identity: AgentIdentity) -> Option<PublicKey> {
    match identity {
        AgentIdentity::PublicKey { key, .. } => Some(key),
        AgentIdentity::Certificate { .. } => None,
    }
}
