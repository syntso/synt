use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::ErrorCode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[error("{message} ({code})")]
pub struct RpcError {
    pub code: ErrorCode,
    pub message: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::PARSE_ERROR, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::INVALID_REQUEST, message)
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::INVALID_PARAMS, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::INTERNAL, message)
    }
}
