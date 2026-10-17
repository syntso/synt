use std::num::NonZeroU16;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_ssh::Target;

use super::{Address, Identity, Login, ResourceName};
use crate::CoreError;
use crate::projects::LocalPath;

const DEFAULT_PORT: u16 = 22;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HostResource {
    pub(crate) address: Address,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) port: Option<NonZeroU16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) user: Option<Login>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) via: Option<ResourceName>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) identity: Option<Identity>,
}

impl HostResource {
    pub(crate) fn of_target(target: &Target) -> Result<Self, CoreError> {
        let identity = target
            .identity_files()
            .first()
            .and_then(|path| LocalPath::of_path(path).ok())
            .map(Identity::File);

        Ok(Self {
            address: Address::parse(target.host())?,
            port: NonZeroU16::new(target.port()),
            user: Login::parse(target.user()).ok(),
            via: None,
            identity,
        })
    }

    pub(crate) fn target(&self) -> Result<Target, CoreError> {
        let user = self
            .user
            .as_ref()
            .map(|user| user.as_str().to_owned())
            .or_else(synt_platform::user_name)
            .ok_or(CoreError::NoLocalUser)?;

        let port = self.port.map_or(DEFAULT_PORT, NonZeroU16::get);
        let target = Target::new(self.address.as_str(), port, user);

        let Some(identity) = &self.identity else {
            return Ok(target);
        };

        let target = target.with_identity_files(identity.files()?);

        match identity.vault_key() {
            Some(name) => Ok(target.with_vault_key(name.as_str())),
            None => Ok(target),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::HostResource;

    fn host(identity: &str) -> Result<HostResource, serde_json::Error> {
        serde_json::from_value(json!({
            "address": "203.0.113.7",
            "user": "root",
            "identity": identity,
        }))
    }

    #[test]
    fn a_secret_identity_names_a_vault_key_and_no_files() {
        let resource = host("secret://night").expect("a host");

        let target = resource.target().expect("target");

        assert_eq!(target.vault_key(), Some("night"));
        assert!(target.identity_files().is_empty());
    }

    #[test]
    fn a_secret_identity_with_a_bad_name_is_refused() {
        assert!(host("secret://../night").is_err());
        assert!(host("secret://").is_err());
    }

    #[test]
    fn a_zero_port_and_a_login_with_a_host_are_refused() {
        let with = |field: &str, value| {
            serde_json::from_value::<HostResource>(json!({ "address": "db", field: value }))
        };

        assert!(with("port", json!(0)).is_err());
        assert!(with("user", json!("root@db")).is_err());
        assert!(with("via", json!("../bastion")).is_err());
        assert!(with("port", json!(2222)).is_ok());
    }
}
