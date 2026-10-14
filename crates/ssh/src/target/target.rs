use std::path::PathBuf;

use super::Spec;
use crate::SshError;
use crate::config::{HostSettings, UserConfig};
use crate::home::local_user;

const DEFAULT_PORT: u16 = 22;
const JUMP_DEPTH_LIMIT: usize = 8;
const HOST_TOKEN: &str = "%h";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
    host: String,
    port: u16,
    user: String,
    identity_files: Vec<PathBuf>,
    vault_key: Option<String>,
    jumps: Vec<Target>,
}

impl Target {
    pub fn new(host: impl Into<String>, port: u16, user: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            user: user.into(),
            identity_files: Vec::new(),
            vault_key: None,
            jumps: Vec::new(),
        }
    }

    pub fn resolve(spec: &str) -> Result<Self, SshError> {
        let config = UserConfig::load()?;

        Self::resolve_with(spec, &config, 0, true)
    }

    #[must_use]
    pub fn with_identity_files(mut self, identity_files: Vec<PathBuf>) -> Self {
        self.identity_files = identity_files;
        self
    }

    #[must_use]
    pub fn with_vault_key(mut self, name: impl Into<String>) -> Self {
        self.vault_key = Some(name.into());
        self
    }

    #[must_use]
    pub fn through(mut self, jumps: Vec<Target>) -> Self {
        self.jumps = jumps;
        self
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn identity_files(&self) -> &[PathBuf] {
        &self.identity_files
    }

    pub fn vault_key(&self) -> Option<&str> {
        self.vault_key.as_deref()
    }

    pub fn jumps(&self) -> &[Target] {
        &self.jumps
    }

    pub fn uses_vault_key(&self, name: &str) -> bool {
        self.vault_key() == Some(name) || self.jumps.iter().any(|jump| jump.uses_vault_key(name))
    }

    pub fn label(&self) -> String {
        format!("{}@{}", self.user, self.address())
    }

    pub(crate) fn previous_hop(&self) -> Option<Target> {
        let (last, earlier) = self.jumps.split_last()?;

        if earlier.is_empty() {
            return Some(last.clone());
        }

        Some(Self {
            jumps: earlier.to_vec(),
            ..last.clone()
        })
    }

    pub(crate) fn address(&self) -> String {
        if self.port == DEFAULT_PORT {
            return self.host.clone();
        }

        format!("{}:{}", self.host, self.port)
    }

    fn resolve_with(
        text: &str,
        config: &UserConfig,
        depth: usize,
        follow_jumps: bool,
    ) -> Result<Self, SshError> {
        if depth > JUMP_DEPTH_LIMIT {
            return Err(SshError::Config(format!(
                "ProxyJump chain through {text:?} is longer than {JUMP_DEPTH_LIMIT} hops"
            )));
        }

        let spec = Spec::parse(text)?;
        let settings = config.settings(&spec.host);

        let jumps = if follow_jumps {
            Self::resolve_jumps(&settings.proxy_jump, config, depth)?
        } else {
            Vec::new()
        };

        Self::from_parts(text, spec, settings, jumps)
    }

    fn resolve_jumps(
        specs: &[String],
        config: &UserConfig,
        depth: usize,
    ) -> Result<Vec<Self>, SshError> {
        specs
            .iter()
            .enumerate()
            .map(|(index, spec)| Self::resolve_with(spec, config, depth + 1, index == 0))
            .collect()
    }

    fn from_parts(
        text: &str,
        spec: Spec,
        settings: HostSettings,
        jumps: Vec<Self>,
    ) -> Result<Self, SshError> {
        let user = spec
            .user
            .or(settings.user)
            .or_else(local_user)
            .ok_or_else(|| SshError::Resolve {
                target: text.to_owned(),
                reason: "no user given and none found in the environment".to_owned(),
            })?;

        let host = settings.host_name.map_or_else(
            || spec.host.clone(),
            |name| name.replace(HOST_TOKEN, &spec.host),
        );
        let port = spec.port.or(settings.port).unwrap_or(DEFAULT_PORT);

        Ok(Self {
            host,
            port,
            user,
            identity_files: settings.identity_files,
            vault_key: None,
            jumps,
        })
    }
}
