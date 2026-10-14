use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ClusterResource, HostResource, TunnelResource};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resource {
    Host(HostResource),
    Tunnel(TunnelResource),
    Cluster(ClusterResource),
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
            Self::Cluster(cluster) => cluster.check(),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Host(_) => "host",
            Self::Tunnel(_) => "tunnel",
            Self::Cluster(_) => "cluster",
        }
    }
}
