use serde::{Deserialize, Serialize};

use super::EndReason;
use crate::{Notification, StreamId};

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

impl SessionEnded {
    pub const METHOD: &'static str = "session.ended";

    pub fn to_notification(&self) -> Result<Notification, serde_json::Error> {
        let params = serde_json::to_value(self)?;
        Ok(Notification::new(Self::METHOD, params))
    }
}
