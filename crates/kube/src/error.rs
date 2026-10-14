use std::io;
use std::time::Duration;

use thiserror::Error;

use crate::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KubeError {
    #[error("kubeconfig {place}: {reason}")]
    Kubeconfig { place: String, reason: String },

    #[error("there is no context {context:?} in kubeconfig {place}")]
    UnknownContext { context: String, place: String },

    #[error("kubeconfig {place} names no current context; say which context to use")]
    NoContext { place: String },

    #[error("cannot connect to the cluster: {0}")]
    Connect(String),

    #[error("the cluster has no kind {0:?}; use a kind, its plural or a short name, such as pods")]
    UnknownKind(String),

    #[error("{0}")]
    Api(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Invalid(String),

    #[error("{0}")]
    Io(String),

    #[error("timed out after {} s", .0.as_secs())]
    Timeout(Duration),

    #[error("stopped before it finished")]
    Stopped,
}

impl KubeError {
    pub fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }

    #[must_use]
    pub(crate) fn concerning(self, object: &ObjectRef) -> Self {
        match self {
            Self::Api(reason) => Self::Api(format!("{object}: {reason}")),
            Self::NotFound(reason) => Self::NotFound(format!("{object}: {reason}")),
            other => other,
        }
    }
}

impl From<kube::Error> for KubeError {
    fn from(error: kube::Error) -> Self {
        match error {
            kube::Error::Api(status) if status.code == 404 => Self::NotFound(status.message),
            kube::Error::Api(status) => {
                Self::Api(api_message(status.code, &status.reason, &status.message))
            }
            other => Self::Connect(other.to_string()),
        }
    }
}

impl From<io::Error> for KubeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub(crate) fn first_line(reason: &impl std::fmt::Display) -> String {
    let reason = reason.to_string();

    reason
        .lines()
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

fn api_message(code: u16, reason: &str, message: &str) -> String {
    if reason.is_empty() {
        return format!("the cluster refused ({code}): {message}");
    }

    format!("the cluster refused ({code} {reason}): {message}")
}
