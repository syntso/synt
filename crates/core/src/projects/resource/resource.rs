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
    pub(crate) fn check(&self) -> Result<(), String> {
        match self {
            Self::Host(host) => host.check(),
            Self::Tunnel(tunnel) => tunnel.check(),
        }
    }
}
