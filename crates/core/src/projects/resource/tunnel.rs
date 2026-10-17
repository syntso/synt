use std::num::NonZeroU16;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_ssh::Destination;

use super::ResourceName;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TunnelResource {
    pub(crate) host: ResourceName,

    #[schemars(with = "String")]
    pub(crate) remote: Destination,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) local: Option<NonZeroU16>,
}
