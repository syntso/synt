use bytes::Bytes;
use synt_platform::Pty;
use synt_ssh::{Algorithms, Shell, TerminalSize};

use super::SessionKind;

const LOCAL_LABEL: &str = "local";

pub(crate) enum Console {
    Remote(Shell),
    Local(Pty),
}

impl Console {
    pub(crate) fn kind(&self) -> SessionKind {
        match self {
            Self::Remote(_) => SessionKind::Ssh,
            Self::Local(_) => SessionKind::Local,
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            Self::Remote(shell) => shell.target().label(),
            Self::Local(_) => LOCAL_LABEL.to_owned(),
        }
    }

    pub(crate) fn algorithms(&self) -> Option<&Algorithms> {
        match self {
            Self::Remote(shell) => shell.algorithms(),
            Self::Local(_) => None,
        }
    }

    pub(crate) async fn write(&self, data: Bytes) -> Result<(), String> {
        match self {
            Self::Remote(shell) => shell.write(data).await.map_err(|error| error.to_string()),
            Self::Local(pty) => pty
                .write(data.to_vec())
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub(crate) async fn resize(&self, size: TerminalSize) -> Result<(), String> {
        match self {
            Self::Remote(shell) => shell.resize(size).await.map_err(|error| error.to_string()),
            Self::Local(pty) => pty
                .resize(size.columns, size.rows)
                .map_err(|error| error.to_string()),
        }
    }

    pub(crate) async fn hang_up(&self) -> bool {
        match self {
            Self::Remote(shell) => shell.hang_up().await.is_ok(),
            Self::Local(pty) => pty.hang_up().is_ok(),
        }
    }

    pub(crate) async fn close(&self) {
        match self {
            Self::Remote(shell) => shell.close().await,
            Self::Local(pty) => pty.stop(),
        }
    }
}
