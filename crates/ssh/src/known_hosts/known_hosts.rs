use std::io;
use std::path::Path;

use russh::keys::{Algorithm, PublicKey};

use super::{Entry, Marker, Verdict};
use crate::SshError;
use crate::home::ssh_directory;

const FILES: [&str; 2] = ["known_hosts", "known_hosts2"];
const DEFAULT_PORT: u16 = 22;

#[derive(Debug, Clone, Default)]
pub(crate) struct KnownHosts {
    entries: Vec<Entry>,
}

impl KnownHosts {
    pub(crate) async fn load() -> Result<Self, SshError> {
        let directory = ssh_directory()?;
        let mut entries = Vec::new();

        for file in FILES {
            let text = read_if_present(&directory.join(file)).await?;
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
            let algorithm = entry.key.algorithm();

            if entry.marker == Marker::None && !algorithms.contains(&algorithm) {
                algorithms.push(algorithm);
            }
        }

        algorithms
    }

    pub(crate) fn verdict(&self, host: &str, port: u16, key: &PublicKey) -> Verdict {
        let matching = self.matching(host, port);

        let revoked = matching
            .iter()
            .any(|entry| entry.marker == Marker::Revoked && same_key(&entry.key, key));
        if revoked {
            return Verdict::Changed;
        }

        let plain: Vec<&PublicKey> = matching
            .iter()
            .filter(|entry| entry.marker == Marker::None)
            .map(|entry| &entry.key)
            .collect();

        if plain.iter().any(|known| same_key(known, key)) {
            return Verdict::Trusted;
        }
        if plain.is_empty() {
            return Verdict::Unknown;
        }
        if plain
            .iter()
            .any(|known| same_family(&known.algorithm(), &key.algorithm()))
        {
            return Verdict::Changed;
        }

        Verdict::OtherTypeKnown
    }

    fn matching(&self, host: &str, port: u16) -> Vec<&Entry> {
        let name = lookup_name(host, port);

        self.entries
            .iter()
            .filter(|entry| entry.matches(&name))
            .collect()
    }
}

pub(crate) fn same_family(known: &Algorithm, offered: &Algorithm) -> bool {
    let both_rsa = matches!(
        (known, offered),
        (Algorithm::Rsa { .. }, Algorithm::Rsa { .. })
    );

    known == offered || both_rsa
}

fn lookup_name(host: &str, port: u16) -> String {
    if port == DEFAULT_PORT {
        return host.to_owned();
    }

    format!("[{host}]:{port}")
}

fn same_key(known: &PublicKey, presented: &PublicKey) -> bool {
    known.key_data() == presented.key_data()
}

async fn read_if_present(path: &Path) -> Result<String, SshError> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}
