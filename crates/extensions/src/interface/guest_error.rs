use std::io;

use synt_actions::ActionError;
use thiserror::Error;

use crate::ExtensionError;
use crate::call::{CallFailure, FailureCode};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {message}")]
pub(crate) struct GuestError {
    code: FailureCode,
    message: String,
}

impl GuestError {
    pub(crate) fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn failed(message: impl Into<String>) -> Self {
        Self::new(FailureCode::Failed, message)
    }

    pub(crate) fn not_allowed(message: impl Into<String>) -> Self {
        Self::new(FailureCode::NotAllowed, message)
    }

    pub(crate) fn missing_capability(extension: &str, missing: &str) -> Self {
        Self::not_allowed(format!("{extension} has no {missing} in its capabilities"))
    }

    pub(crate) fn unsupported(entry: &str) -> Self {
        Self::new(
            FailureCode::Unsupported,
            format!("extensions for api 0.1 have no {entry}"),
        )
    }

    pub(crate) fn code(&self) -> &FailureCode {
        &self.code
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl From<io::Error> for GuestError {
    fn from(error: io::Error) -> Self {
        Self::failed(error.to_string())
    }
}

impl From<ExtensionError> for GuestError {
    fn from(error: ExtensionError) -> Self {
        Self::failed(error.to_string())
    }
}

impl From<GuestError> for ActionError {
    fn from(error: GuestError) -> Self {
        Self::failed(error.to_string())
    }
}

impl From<CallFailure> for GuestError {
    fn from(failure: CallFailure) -> Self {
        Self::new(failure.code().clone(), failure.message())
    }
}
