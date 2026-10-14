use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ssh2_config::{HostParams, ParseRule, SshConfig};

use super::HostSettings;
use crate::SshError;
use crate::locked::locked;
use crate::ssh_directory::SshDirectory;

const CONFIG_FILE: &str = "config";
const NO_JUMP: &str = "none";
const INCLUDE: &[u8] = b"include";

type Remembered = (PathBuf, Vec<u8>, Arc<SshConfig>);

static REMEMBERED: Mutex<Option<Remembered>> = Mutex::new(None);

pub(crate) struct UserConfig {
    parsed: Arc<SshConfig>,
}

impl UserConfig {
    pub(crate) fn load() -> Result<Self, SshError> {
        let path = SshDirectory::User.path()?.join(CONFIG_FILE);

        let text = match fs::read(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::empty()),
            Err(error) => return Err(error.into()),
        };

        if let Some(parsed) = recall(&path, &text) {
            return Ok(Self { parsed });
        }

        let config = Self::parse(&path, &text)?;

        if !includes_other_files(&text) {
            remember(path, text, Arc::clone(&config.parsed));
        }

        Ok(config)
    }

    pub(crate) fn parse(path: &Path, text: &[u8]) -> Result<Self, SshError> {
        let rules = ParseRule::ALLOW_UNKNOWN_FIELDS | ParseRule::ALLOW_UNSUPPORTED_FIELDS;

        let parsed = SshConfig::default()
            .parse(&mut &text[..], rules)
            .map_err(|error| SshError::Config(format!("{}: {error}", path.display())))?;

        Ok(Self {
            parsed: Arc::new(parsed),
        })
    }

    pub(crate) fn settings(&self, host: &str) -> HostSettings {
        let params = self.parsed.query(host);

        HostSettings {
            host_name: params.host_name.clone(),
            user: params.user.clone(),
            port: params.port,
            identity_files: params.identity_file.clone().unwrap_or_default(),
            proxy_jump: proxy_jump(&params),
        }
    }

    pub(crate) fn aliases(&self) -> Vec<String> {
        self.parsed
            .get_hosts()
            .iter()
            .flat_map(|host| &host.pattern)
            .filter(|clause| !clause.negated && !is_pattern(&clause.pattern))
            .map(|clause| clause.pattern.clone())
            .collect()
    }

    fn empty() -> Self {
        Self {
            parsed: Arc::new(SshConfig::default()),
        }
    }
}

fn recall(path: &Path, text: &[u8]) -> Option<Arc<SshConfig>> {
    let remembered = locked(&REMEMBERED);
    let (known_path, known_text, parsed) = remembered.as_ref()?;

    (known_path == path && known_text == text).then(|| Arc::clone(parsed))
}

fn remember(path: PathBuf, text: Vec<u8>, parsed: Arc<SshConfig>) {
    let mut remembered = locked(&REMEMBERED);

    *remembered = Some((path, text, parsed));
}

fn is_pattern(alias: &str) -> bool {
    alias.contains(['*', '?', '!'])
}

fn includes_other_files(text: &[u8]) -> bool {
    text.windows(INCLUDE.len())
        .any(|window| window.eq_ignore_ascii_case(INCLUDE))
}

fn proxy_jump(params: &HostParams) -> Vec<String> {
    let hops: Vec<String> = params
        .proxy_jump
        .iter()
        .flatten()
        .flat_map(|entry| entry.split(','))
        .map(str::trim)
        .filter(|hop| !hop.is_empty())
        .map(str::to_owned)
        .collect();

    if hops
        .first()
        .is_some_and(|hop| hop.eq_ignore_ascii_case(NO_JUMP))
    {
        return Vec::new();
    }

    hops
}
