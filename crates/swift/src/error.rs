use synt_client::ClientError;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SyntError {
    #[error("core refused ({code}): {message}")]
    Refused { code: i64, message: String },

    #[error("the connection to the core is closed")]
    Disconnected,

    #[error("not valid JSON: {reason}")]
    InvalidJson { reason: String },

    #[error("{reason}")]
    Failed { reason: String },
}

impl From<ClientError> for SyntError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Rpc(error) => Self::Refused {
                code: error.code.0,
                message: error.message,
            },
            ClientError::Disconnected => Self::Disconnected,
            other => Self::Failed {
                reason: other.to_string(),
            },
        }
    }
}

impl From<serde_json::Error> for SyntError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidJson {
            reason: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use synt_client::ClientError;
    use synt_proto::{ErrorCode, RpcError};

    use super::SyntError;

    #[test]
    fn refusals_keep_their_code() {
        let refused = ClientError::Rpc(RpcError::new(ErrorCode::DENIED, "read only"));

        let error = SyntError::from(refused);

        assert!(matches!(
            error,
            SyntError::Refused { code, message } if code == ErrorCode::DENIED.0 && message == "read only"
        ));
    }

    #[test]
    fn a_closed_connection_is_disconnected() {
        let error = SyntError::from(ClientError::Disconnected);

        assert!(matches!(error, SyntError::Disconnected));
    }
}
