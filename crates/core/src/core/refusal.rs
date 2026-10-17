use synt_actions::ActionError;
use synt_proto::{ErrorCode, RpcError};

pub(crate) enum Refusal {
    Failed(RpcError),
    Denied(String),
}

impl From<ActionError> for Refusal {
    fn from(error: ActionError) -> Self {
        let code = match error {
            ActionError::InvalidParams(_) | ActionError::TargetRequired => {
                ErrorCode::INVALID_PARAMS
            }
            ActionError::Failed(_) => ErrorCode::ACTION_FAILED,
            ActionError::InvalidId { .. } | ActionError::Duplicate(_) => ErrorCode::INTERNAL,
        };

        Self::Failed(RpcError::new(code, error.to_string()))
    }
}
