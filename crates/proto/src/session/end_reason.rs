use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Exited,
    Closed,
    ConnectionLost,
    CoreStopped,
}

impl EndReason {
    pub fn describe(self) -> &'static str {
        match self {
            Self::Exited => "the shell exited",
            Self::Closed => "the session was closed",
            Self::ConnectionLost => "the connection was lost",
            Self::CoreStopped => "the core stopped",
        }
    }
}
