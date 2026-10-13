use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_ssh::Target;

use crate::projects::names::check_name;

const DEFAULT_PORT: u16 = 22;
const AGENT: &str = "agent";
const SECRET_PREFIX: &str = "secret://";
const HOME_PREFIX: &str = "~/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostResource {
    pub address: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
}

impl HostResource {
    pub(crate) fn check(&self) -> Result<(), String> {
        if !is_plain_word(&self.address) {
            return Err(format!(
                "address must be a host name or address, got {:?}",
                self.address
            ));
        }

        if self.port == Some(0) {
            return Err("port must be between 1 and 65535".to_owned());
        }

        if let Some(user) = &self.user
            && (!is_plain_word(user) || user.contains('@'))
        {
            return Err(format!("user must be a login name, got {user:?}"));
        }

        if let Some(via) = &self.via {
            check_name(via).map_err(|reason| format!("via: {reason}"))?;
        }

        self.identity_files().map(|_| ())
    }

    pub(crate) fn target(&self) -> Result<Target, String> {
        let user = self
            .user
            .clone()
            .or_else(synt_ssh::local_user)
            .ok_or("no user given and none found in the environment")?;

        let target = Target::new(&self.address, self.port.unwrap_or(DEFAULT_PORT), user)
            .with_identity_files(self.identity_files()?);

        Ok(target)
    }

    fn identity_files(&self) -> Result<Vec<PathBuf>, String> {
        let Some(identity) = self.identity.as_deref() else {
            return Ok(Vec::new());
        };

        if identity == AGENT {
            return Ok(Vec::new());
        }

        if identity.starts_with(SECRET_PREFIX) {
            return Err(format!(
                "identity {identity:?}: secret references are not supported yet; \
                 use \"agent\" or the path of a key file"
            ));
        }

        if let Some(rest) = identity.strip_prefix(HOME_PREFIX) {
            let home = synt_platform::home_directory()
                .map_err(|error| format!("there is no home folder to expand ~ in: {error}"))?;
            return Ok(vec![home.join(rest)]);
        }

        let path = PathBuf::from(identity);

        if !path.is_absolute() {
            return Err(format!(
                "identity {identity:?} must be \"agent\", an absolute path, or a path starting with ~/"
            ));
        }

        Ok(vec![path])
    }
}

fn is_plain_word(text: &str) -> bool {
    !text.is_empty()
        && !text.contains(|character: char| character.is_whitespace() || character.is_control())
}
