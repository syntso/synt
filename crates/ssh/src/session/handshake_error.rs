use thiserror::Error;

use crate::SshError;

#[derive(Debug, Error)]
pub(crate) enum HandshakeError {
    #[error(transparent)]
    Protocol(#[from] russh::Error),

    #[error(transparent)]
    Rejected(SshError),
}
