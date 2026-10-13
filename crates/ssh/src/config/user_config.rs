use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use ssh2_config::{HostParams, ParseRule, SshConfig};

use super::HostSettings;
use crate::SshError;
use crate::home::ssh_directory;

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
        let path = ssh_directory()?.join(CONFIG_FILE);

        let text = match fs::read(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::empty()),
            Err(error) => return Err(error.into()),
        };

        if let Some(parsed) = recall(&path, &text) {
            return Ok(Self { parsed });
        }

        let parsed = Arc::new(parse(&path, &text)?);

        if !includes_other_files(&text) {
            remember(path, text, Arc::clone(&parsed));
        }

        Ok(Self { parsed })
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

    fn empty() -> Self {
        Self {
            parsed: Arc::new(SshConfig::default()),
        }
    }
}

fn parse(path: &Path, text: &[u8]) -> Result<SshConfig, SshError> {
    let rules = ParseRule::ALLOW_UNKNOWN_FIELDS | ParseRule::ALLOW_UNSUPPORTED_FIELDS;

    SshConfig::default()
        .parse(&mut &text[..], rules)
        .map_err(|error| SshError::Config(format!("{}: {error}", path.display())))
}

fn recall(path: &Path, text: &[u8]) -> Option<Arc<SshConfig>> {
    let remembered = REMEMBERED.lock().unwrap_or_else(PoisonError::into_inner);
    let (known_path, known_text, parsed) = remembered.as_ref()?;

    (known_path == path && known_text == text).then(|| Arc::clone(parsed))
}

fn remember(path: PathBuf, text: Vec<u8>, parsed: Arc<SshConfig>) {
    let mut remembered = REMEMBERED.lock().unwrap_or_else(PoisonError::into_inner);

    *remembered = Some((path, text, parsed));
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
