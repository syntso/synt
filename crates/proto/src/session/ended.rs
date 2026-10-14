use serde::{Deserialize, Serialize};

use super::{EndReason, SessionId};
use crate::{Notice, StreamId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEnded {
    pub session: SessionId,
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
