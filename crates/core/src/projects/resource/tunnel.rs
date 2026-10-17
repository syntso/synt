use std::num::NonZeroU16;

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
    pub local: Option<NonZeroU16>,
}

impl TunnelResource {
    pub(crate) fn destination(&self) -> Result<Destination, CoreError> {
        Destination::parse(&self.remote).map_err(|_| CoreError::NotARemote(self.remote.clone()))
    }
}

impl ResourceSpec for TunnelResource {
    fn kind(&self) -> &'static str {
        "tunnel"
    }

    fn check(&self) -> Result<(), CoreError> {
        ResourceName::check(&self.host, "host")?;

        self.destination().map(drop)
    }
}
