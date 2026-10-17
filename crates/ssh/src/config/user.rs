use std::fs;
use std::path::Path;
use std::sync::Arc;

use ssh2_config::{ParseRule, SshConfig};
use synt_platform::Found;

use super::{HostSettings, Remembered};
use crate::SshError;
use crate::directory::SshDirectory;

const CONFIG_FILE: &str = "config";
const INCLUDE: &[u8] = b"include";
const PATTERN_SIGNS: [char; 3] = ['*', '?', '!'];

pub struct UserConfig {
    parsed: Arc<SshConfig>,
}

impl UserConfig {
    pub fn load() -> Result<Self, SshError> {
        let path = SshDirectory::User.path()?.join(CONFIG_FILE);

        let Some(text) = fs::read(&path).found()? else {
            return Ok(Self::empty());
        };

        if let Some(parsed) = Remembered::recall(&path, &text) {
            return Ok(Self { parsed });
        }

        let config = Self::parse(&path, &text)?;

        if !Self::includes_other_files(&text) {
            Remembered::keep(path, text, Arc::clone(&config.parsed));
        }

        Ok(config)
    }

    pub fn aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self
            .parsed
            .get_hosts()
            .iter()
            .flat_map(|host| &host.pattern)
            .filter(|clause| !clause.negated && !clause.pattern.contains(PATTERN_SIGNS))
            .map(|clause| clause.pattern.clone())
            .collect();

        aliases.sort();
        aliases.dedup();

        aliases
    }

    pub(crate) fn parse(path: &Path, text: &[u8]) -> Result<Self, SshError> {
        let rules = ParseRule::ALLOW_UNKNOWN_FIELDS | ParseRule::ALLOW_UNSUPPORTED_FIELDS;

        let parsed = SshConfig::default()
            .parse(&mut &text[..], rules)
            .map_err(|cause| SshError::Config {
                path: path.to_owned(),
                cause: Arc::new(cause),
            })?;

        Ok(Self {
            parsed: Arc::new(parsed),
        })
    }

    pub(crate) fn settings(&self, host: &str) -> HostSettings {
        HostSettings::from(&self.parsed.query(host))
    }

    fn empty() -> Self {
        Self {
            parsed: Arc::new(SshConfig::default()),
        }
    }

    fn includes_other_files(text: &[u8]) -> bool {
        text.windows(INCLUDE.len())
            .any(|window| window.eq_ignore_ascii_case(INCLUDE))
    }
}
