use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::CoreError;
use crate::projects::LocalPath;
use crate::ssh_keys::KeyName;

const AGENT: &str = "agent";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub(crate) enum Identity {
    Agent,
    Vault(KeyName),
    File(LocalPath),
}

impl Identity {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        if text == AGENT {
            return Ok(Self::Agent);
        }

        if let Some(named) = KeyName::from_reference(text) {
            let name = named.map_err(|reason| CoreError::IdentityName {
                identity: text.to_owned(),
                reason: Box::new(reason.into()),
            })?;

            return Ok(Self::Vault(name));
        }

        let path = LocalPath::parse(text).map_err(|_| CoreError::NotAnIdentity(text.to_owned()))?;

        Ok(Self::File(path))
    }

    pub(crate) fn vault_key(&self) -> Option<&KeyName> {
        match self {
            Self::Vault(name) => Some(name),
            Self::Agent | Self::File(_) => None,
        }
    }

    pub(crate) fn files(&self) -> Result<Vec<PathBuf>, CoreError> {
        match self {
            Self::File(path) => Ok(vec![path.path()?]),
            Self::Agent | Self::Vault(_) => Ok(Vec::new()),
        }
    }
}

impl TryFrom<String> for Identity {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text)
    }
}

impl From<Identity> for String {
    fn from(identity: Identity) -> Self {
        match identity {
            Identity::Agent => AGENT.to_owned(),
            Identity::Vault(name) => name.reference(),
            Identity::File(path) => path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Identity;

    #[test]
    fn an_identity_is_the_agent_a_vault_key_or_a_file() {
        assert_eq!(Identity::parse("agent").unwrap(), Identity::Agent);
        assert_eq!(
            Identity::parse("secret://night")
                .unwrap()
                .vault_key()
                .map(|name| name.as_str().to_owned()),
            Some("night".to_owned())
        );
        assert!(matches!(
            Identity::parse("/home/deploy/.ssh/id_ed25519").unwrap(),
            Identity::File(_)
        ));

        assert!(Identity::parse("secret://../night").is_err());
        assert!(Identity::parse("secret://").is_err());
        assert!(Identity::parse("id_ed25519").is_err());
    }
}
