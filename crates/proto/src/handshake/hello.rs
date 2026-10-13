use serde::{Deserialize, Serialize};

use super::{ClientKind, ProtocolVersion};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: ProtocolVersion,
    pub client: ClientKind,
    pub software: String,
}

impl Hello {
    pub fn new(client: ClientKind, software: impl Into<String>) -> Self {
        Self {
            protocol: ProtocolVersion::CURRENT,
            client,
            software: software.into(),
        }
    }
}
