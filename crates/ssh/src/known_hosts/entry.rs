use russh::keys::PublicKey;

use super::{HostPatterns, Marker};

#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) marker: Marker,
    pub(super) key: PublicKey,
    patterns: HostPatterns,
}

impl Entry {
    pub(super) fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let mut tokens = line.split_whitespace().peekable();
        let marker = match tokens
            .peek()
            .copied()
            .filter(|token| token.starts_with('@'))
        {
            Some(token) => {
                tokens.next();
                Marker::parse(token)?
            }
            None => Marker::None,
        };

        let patterns = HostPatterns::new(tokens.next()?);
        let key_type = tokens.next()?;
        let key_data = tokens.next()?;
        let key = PublicKey::from_openssh(&format!("{key_type} {key_data}")).ok()?;

        Some(Self {
            marker,
            key,
            patterns,
        })
    }

    pub(super) fn matches(&self, name: &str) -> bool {
        self.patterns.matches(name)
    }
}
