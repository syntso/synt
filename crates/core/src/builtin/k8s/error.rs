use synt_actions::ActionError;
use synt_journal::{ResourceRef, Seq};
use synt_kube::KubeError;
use thiserror::Error;

use super::ForwardId;
use crate::CoreError;

#[derive(Debug, Error)]
pub(crate) enum K8sError {
    #[error(transparent)]
    K8sActions(#[from] KubeError),

    #[error(transparent)]
    Action(#[from] ActionError),

    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("{0} is not a cluster")]
    NotACluster(ResourceRef),

    #[error("{0}")]
    Place(KubeError),

    #[error("the cluster can only be reached from inside the core")]
    OutsideCore,

    #[error("reading the kubeconfig stopped unexpectedly")]
    KubeconfigStopped,

    #[error(
        "{0} names secrets now, which was not known when the call was admitted; \
         run it again so it is asked as privileged"
    )]
    SecretsNow(String),

    #[error(
        "{0} is cluster-scoped, which was not known when the call was admitted; \
         run it again so it is asked as privileged"
    )]
    ClusterScopedNow(String),

    #[error("follow and stream go together: name the stream to follow the logs on")]
    FollowWithoutStream,

    #[error("no stream was started by request {0}")]
    NoStream(Seq),

    #[error("the stream of request {0} belongs to another caller")]
    StreamOfAnother(Seq),

    #[error("this call has no connection to stream over")]
    NoConnection,

    #[error("there is no open port forward {0}")]
    NoForward(ForwardId),

    #[error(
        "{0} is now reached through a root login, which the call was not admitted for; \
         run it again so it is asked as privileged"
    )]
    RootLoginNow(ResourceRef),

    #[error(
        "{target} now leads to {destination}, which is not what the call was admitted for; \
         run it again so it is checked for the new place"
    )]
    DestinationChanged {
        target: ResourceRef,
        destination: String,
    },
}

impl From<K8sError> for ActionError {
    fn from(error: K8sError) -> Self {
        match error {
            K8sError::K8sActions(KubeError::Invalid(reason)) => Self::InvalidParams(reason),
            K8sError::K8sActions(error @ KubeError::UnknownKind(_)) => {
                Self::InvalidParams(error.to_string())
            }
            K8sError::Action(error) => error,
            K8sError::Core(error) => error.into(),
            K8sError::FollowWithoutStream => Self::InvalidParams(error.to_string()),
            other => Self::Failed(other.to_string()),
        }
    }
}
