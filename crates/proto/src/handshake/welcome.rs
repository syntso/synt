use serde::{Deserialize, Serialize};

use super::ProtocolVersion;
use crate::Requester;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    pub protocol: ProtocolVersion,
    pub core: String,
    pub actor: Requester,
}
