use std::fs::File;
use std::io::{self, BufReader};

use ssh2_config::{HostParams, ParseRule, SshConfig};

use super::HostSettings;
use crate::SshError;
use crate::home::ssh_directory;

const CONFIG_FILE: &str = "config";
const NO_JUMP: &str = "none";

pub(crate) struct UserConfig {
    parsed: SshConfig,
}

impl UserConfig {
    pub(crate) fn load() -> Result<Self, SshError> {
        let path = ssh_directory()?.join(CONFIG_FILE);

        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::empty()),
            Err(error) => return Err(error.into()),
        };

        let rules = ParseRule::ALLOW_UNKNOWN_FIELDS | ParseRule::ALLOW_UNSUPPORTED_FIELDS;
        let parsed = SshConfig::default()
            .parse(&mut BufReader::new(file), rules)
            .map_err(|error| SshError::Config(format!("{}: {error}", path.display())))?;

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
            parsed: SshConfig::default(),
        }
    }
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
