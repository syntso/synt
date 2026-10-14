use std::fmt::{self, Display, Formatter};
use std::io;

use synt_actions::ActionError;

use crate::ExtensionError;
use crate::call::CallFailure;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestError {
    pub(crate) code: String,
    pub(crate) message: String,
}

impl GuestError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub(crate) fn failed(message: impl Into<String>) -> Self {
        Self::new("failed", message)
    }

    pub(crate) fn not_allowed(message: impl Into<String>) -> Self {
        Self::new("not_allowed", message)
    }

    pub(crate) fn unsupported(entry: &str) -> Self {
        Self::new(
            "unsupported",
            format!("extensions for api 0.1 have no {entry}"),
        )
    }
}

impl Display for GuestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
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
        Self::new(failure.code, failure.message)
    }
}
