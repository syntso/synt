use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_ssh::Destination;

use super::{ResourceName, ResourceSpec};
use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TunnelResource {
    pub host: String,
    pub remote: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<u16>,
}

impl TunnelResource {
    pub(crate) fn destination(&self) -> Result<Destination, CoreError> {
        let invalid = || CoreError::NotARemote(self.remote.clone());

        let (host, port) = self.remote.rsplit_once(':').ok_or_else(invalid)?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let port: u16 = port.parse().map_err(|_| invalid())?;

        if host.is_empty() || port == 0 || host.contains(char::is_whitespace) {
            return Err(invalid());
        }

        Ok(Destination::new(host, port))
    }
}

impl ResourceSpec for TunnelResource {
    fn kind(&self) -> &'static str {
        "tunnel"
    }

    fn check(&self) -> Result<(), CoreError> {
        ResourceName::check(&self.host, "host")?;

        if self.local == Some(0) {
            return Err(CoreError::PortZero("local"));
        }

        self.destination().map(drop)
    }
}
