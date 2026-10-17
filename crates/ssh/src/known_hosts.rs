mod entry;
mod family;
mod hashed_host;
mod host_patterns;
mod marker;
mod verdict;

use std::path::Path;

use russh::keys::{Algorithm, PublicKey};
use synt_platform::Found;

use entry::Entry;
pub(crate) use family::Family;
use hashed_host::HashedHost;
use host_patterns::HostPatterns;
use marker::Marker;
pub(crate) use verdict::Verdict;

use crate::directory::SshDirectory;
use crate::{SshError, Target};

const FILES: [&str; 2] = ["known_hosts", "known_hosts2"];
const DEFAULT_PORT: u16 = 22;

#[derive(Debug, Clone, Default)]
pub(crate) struct KnownHosts {
    entries: Vec<Entry>,
}

impl KnownHosts {
    pub(crate) async fn load(directory: &SshDirectory) -> Result<Self, SshError> {
        let directory = directory.path()?;
        let mut entries = Vec::new();

        for file in FILES {
            let text = Self::read_if_present(&directory.join(file)).await?;
            entries.extend(Self::parse(&text).entries);
        }

        Ok(Self { entries })
    }

    pub(crate) fn parse(text: &str) -> Self {
        let entries = text.lines().filter_map(Entry::parse).collect();

        Self { entries }
    }

    pub(crate) fn known_algorithms(&self, host: &str, port: u16) -> Vec<Algorithm> {
        let mut algorithms: Vec<Algorithm> = Vec::new();

        for entry in self.matching(host, port) {
            let algorithm = entry.key().algorithm();

            if entry.is_plain() && !algorithms.contains(&algorithm) {
                algorithms.push(algorithm);
            }
        }

        algorithms
    }

    pub(crate) fn verdict(&self, host: &str, port: u16, key: &PublicKey) -> Verdict {
        let matching = self.matching(host, port);

        let revoked = matching
            .iter()
            .any(|entry| entry.is_revoked() && entry.holds(key));

        if revoked {
            return Verdict::Changed;
        }

        let plain: Vec<&Entry> = matching
            .into_iter()
            .filter(|entry| entry.is_plain())
            .collect();

        if plain.iter().any(|entry| entry.holds(key)) {
            return Verdict::Trusted;
        }

        if plain.is_empty() {
            return Verdict::Unknown;
        }

        let same_family = plain
            .iter()
            .any(|entry| entry.key().algorithm().same_family(&key.algorithm()));

        if same_family {
            return Verdict::Changed;
        }

        Verdict::OtherTypeKnown
    }

    pub(crate) fn require_known(&self, target: &Target) -> Result<(), SshError> {
        let unknown = target
            .hops()
            .into_iter()
            .find(|hop| self.known_algorithms(hop.host(), hop.port()).is_empty());

        match unknown {
            Some(hop) => Err(SshError::NotKnownHost {
                host: hop.address(),
            }),
            None => Ok(()),
        }
    }

    fn matching(&self, host: &str, port: u16) -> Vec<&Entry> {
        let name = Self::lookup_name(host, port);

        self.entries
            .iter()
            .filter(|entry| entry.matches(&name))
            .collect()
    }

    fn lookup_name(host: &str, port: u16) -> String {
        if port == DEFAULT_PORT {
            return host.to_owned();
        }

        format!("[{host}]:{port}")
    }

    async fn read_if_present(path: &Path) -> Result<String, SshError> {
        let text = tokio::fs::read_to_string(path).await.found()?;

        Ok(text.unwrap_or_default())
    }
}
