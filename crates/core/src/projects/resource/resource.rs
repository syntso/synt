use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{HostResource, TunnelResource};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resource {
    Host(HostResource),
    Tunnel(TunnelResource),
}

impl Resource {
    pub(crate) fn from_toml(text: &str) -> Result<Self, String> {
        let resource: Self = toml::from_str(text).map_err(|error| error.message().to_owned())?;

        resource.check()?;
        Ok(resource)
    }

    pub(crate) fn to_toml(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|error| error.to_string())
    }

    pub(crate) fn check(&self) -> Result<(), String> {
        match self {
            Self::Host(host) => host.check(),
            Self::Tunnel(tunnel) => tunnel.check(),
        }
    }
}
