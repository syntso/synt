use serde::{Deserialize, Serialize};

use super::EndReason;
use crate::{Notice, StreamId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEnded {
    pub session: u64,
    pub stream: StreamId,
    pub reason: EndReason,

    #[serde(default)]
    pub exit: Option<u32>,

    #[serde(default)]
    pub signal: Option<String>,

    pub duration_ms: u64,
}

impl Notice for SessionEnded {
    const METHOD: &'static str = "session.ended";
}
