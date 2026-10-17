use std::time::Instant;

use crate::session::Connection;
use crate::{ConnectionState, SshError};

pub(super) enum Phase {
    Idle,
    Connecting,
    Connected(Connection),
    Lost,
    Reconnecting,
    Failed(SshError, Instant),
}

impl Phase {
    pub(super) fn state(&self) -> ConnectionState {
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

    pub(super) fn dialing(&self) -> Self {
        match self {
            Self::Lost | Self::Reconnecting | Self::Connected(_) => Self::Reconnecting,
            Self::Idle | Self::Connecting | Self::Failed(..) => Self::Connecting,
        }
    }

    pub(super) fn abandoned(&self) -> Self {
        match self {
            Self::Reconnecting | Self::Lost | Self::Connected(_) => Self::Lost,
            Self::Idle | Self::Connecting | Self::Failed(..) => Self::Idle,
        }
    }

    pub(super) fn connection(&self) -> Option<&Connection> {
        match self {
            Self::Connected(connection) => Some(connection),
            _ => None,
        }
    }

    pub(super) fn error(&self) -> Option<&SshError> {
        match self {
            Self::Failed(error, _) => Some(error),
            _ => None,
        }
    }
}
