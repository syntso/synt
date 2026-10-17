use std::path::PathBuf;
use std::vec;

use russh::keys::PublicKey;

use super::Candidate;

#[derive(Debug, Default)]
pub(super) struct Candidates {
    list: Vec<Candidate>,
}

impl Candidates {
    pub(super) fn vault_only(name: &str) -> Self {
        Self {
            list: vec![Candidate::Vault(name.to_owned())],
        }
    }

    pub(super) async fn from_files(paths: &[PathBuf], held: &[PublicKey]) -> Self {
        let mut candidates = Self::default();

        for path in paths {
            let Some(candidate) = Candidate::from_file(path, held).await else {
                continue;
            };

            let duplicate = candidate
                .public_key()
                .is_some_and(|key| candidates.holds(key));

            if !duplicate {
                candidates.list.push(candidate);
            }
        }

        candidates
    }

    pub(super) fn add_agent_keys(&mut self, held: Vec<PublicKey>) {
        for key in held {
            if !self.holds(&key) {
                self.list.push(Candidate::Agent(key));
            }
        }
    }

    pub(super) fn put_first(&mut self, key: &PublicKey) {
        let Some(position) = self.list.iter().position(|known| known.holds(key)) else {
            return;
        };

        self.list[..=position].rotate_right(1);
    }

    pub(super) fn truncate(&mut self, most: usize) {
        self.list.truncate(most);
    }

    fn holds(&self, key: &PublicKey) -> bool {
        self.list.iter().any(|known| known.holds(key))
    }
}

impl IntoIterator for Candidates {
    type Item = Candidate;
    type IntoIter = vec::IntoIter<Candidate>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.into_iter()
    }
}
