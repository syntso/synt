use std::time::Instant;

use super::ConnectionState;
use crate::SshError;
use crate::session::Connection;

pub(crate) enum Phase {
    Idle,
    Connecting,
    Connected(Connection),
    Lost,
    Reconnecting,
    Failed(SshError, Instant),
}

impl Phase {
    pub(crate) fn state(&self) -> ConnectionState {
        match self {
            Self::Idle => ConnectionState::Idle,
            Self::Connecting => ConnectionState::Connecting,
            Self::Connected(connection) if connection.is_closed() => ConnectionState::Lost,
            Self::Connected(_) => ConnectionState::Connected,
            Self::Lost => ConnectionState::Lost,
            Self::Reconnecting => ConnectionState::Reconnecting,
            Self::Failed(..) => ConnectionState::Failed,
        }
    }

    pub(crate) fn dialing(&self) -> Self {
        match self {
            Self::Lost | Self::Reconnecting | Self::Connected(_) => Self::Reconnecting,
            Self::Idle | Self::Connecting | Self::Failed(..) => Self::Connecting,
        }
    }

    pub(crate) fn abandoned(&self) -> Self {
        match self {
            Self::Reconnecting | Self::Lost | Self::Connected(_) => Self::Lost,
            Self::Idle | Self::Connecting | Self::Failed(..) => Self::Idle,
        }
    }

    pub(crate) fn error(&self) -> Option<&SshError> {
        match self {
            Self::Failed(error, _) => Some(error),
            _ => None,
        }
    }
}
