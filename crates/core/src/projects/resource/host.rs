use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_ssh::Target;

use crate::projects::local_path::local_path;
use crate::projects::names::check_name;
use crate::ssh_keys::KeyName;

const DEFAULT_PORT: u16 = 22;
const AGENT: &str = "agent";

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

        self.vault_key()?;
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

        match self.vault_key()? {
            Some(name) => Ok(target.with_vault_key(name.as_str())),
            None => Ok(target),
        }
    }

    fn vault_key(&self) -> Result<Option<KeyName>, String> {
        let Some(identity) = self.identity.as_deref() else {
            return Ok(None);
        };

        KeyName::from_reference(identity).transpose()
    }

    fn identity_files(&self) -> Result<Vec<PathBuf>, String> {
        let Some(identity) = self.identity.as_deref() else {
            return Ok(Vec::new());
        };

        if identity == AGENT || KeyName::from_reference(identity).is_some() {
            return Ok(Vec::new());
        }

        let path = local_path(identity).map_err(|_| {
            format!(
                "identity {identity:?} must be \"agent\", \"secret://<name>\", an absolute path, \
                 or a path starting with ~/"
            )
        })?;

        Ok(vec![path])
    }
}

fn is_plain_word(text: &str) -> bool {
    !text.is_empty()
        && !text.contains(|character: char| character.is_whitespace() || character.is_control())
}

#[cfg(test)]
mod tests {
    use super::HostResource;

    fn host(identity: &str) -> HostResource {
        HostResource {
            address: "203.0.113.7".to_owned(),
            port: None,
            user: Some("root".to_owned()),
            via: None,
            identity: Some(identity.to_owned()),
        }
    }

    #[test]
    fn a_secret_identity_names_a_vault_key_and_no_files() {
        let resource = host("secret://night");

        let target = resource.target().expect("target");

        assert!(resource.check().is_ok());
        assert_eq!(target.vault_key(), Some("night"));
        assert!(target.identity_files().is_empty());
    }

    #[test]
    fn a_secret_identity_with_a_bad_name_is_refused() {
        assert!(host("secret://../night").check().is_err());
        assert!(host("secret://").target().is_err());
    }
}
