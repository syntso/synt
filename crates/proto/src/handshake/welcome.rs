use serde::{Deserialize, Serialize};

use super::ProtocolVersion;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    pub protocol: ProtocolVersion,
    pub core: String,
}
