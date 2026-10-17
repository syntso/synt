use std::error::Error as StdError;
use std::fmt::Display;
use std::io;
use std::time::Duration;

use thiserror::Error;
use tokio::task::JoinError;

use crate::ObjectRef;

#[derive(Debug, Error)]
pub enum KubeError {
    #[error("kubeconfig {place}: {reason}")]
    Kubeconfig { place: String, reason: String },

    #[error("there is no context {context:?} in kubeconfig {place}")]
    UnknownContext { context: String, place: String },

    #[error("kubeconfig {place} names no current context; say which context to use")]
    NoContext { place: String },

    #[error("cannot connect to the cluster: {0}")]
    Connect(#[source] kube::Error),

    #[error("cannot connect to the cluster: {0}")]
    Attach(#[source] Box<dyn StdError + Send + Sync>),

    #[error("cannot connect to the cluster: reading the kubeconfig stopped unexpectedly")]
    Interrupted(#[from] JoinError),

    #[error("the cluster has no kind {0:?}; use a kind, its plural or a short name, such as pods")]
    UnknownKind(String),

    #[error("{0}")]
    Api(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Invalid(String),

    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("cannot listen on 127.0.0.1:{port}: {source}")]
    Listen {
        port: u16,
        #[source]
        source: io::Error,
    },

    #[error("the cluster opened no stream for port {0}")]
    NoStream(u16),

    #[error("timed out after {} s", .0.as_secs())]
    Timeout(Duration),

    #[error("stopped before it finished")]
    Stopped,
}

impl KubeError {
    pub fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }

    pub(crate) fn kubeconfig(place: impl Display, reason: impl Display) -> Self {
        Self::Kubeconfig {
            place: place.to_string(),
            reason: first_line(&reason),
        }
    }

    pub(crate) fn manifest_not_yaml(reason: impl Display) -> Self {
        Self::invalid(format!("the manifest is not YAML: {}", first_line(&reason)))
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
            other => Self::Connect(other),
        }
    }
}

fn first_line(reason: &impl Display) -> String {
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
