#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Lost,
    Reconnecting,
    Failed,
}

impl ConnectionState {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Lost => "lost",
            Self::Reconnecting => "reconnecting",
            Self::Failed => "failed",
        }
    }
}
