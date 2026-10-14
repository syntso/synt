use synt_actions::ActionError;
use synt_kube::KubeError;

pub(crate) fn failed(error: KubeError) -> ActionError {
    match error {
        KubeError::Invalid(reason) => ActionError::InvalidParams(reason),
        KubeError::UnknownKind(_) => ActionError::InvalidParams(error.to_string()),
        other => ActionError::Failed(other.to_string()),
    }
}
