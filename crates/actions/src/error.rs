use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ActionError {
    #[error("invalid parameters: {0}")]
    InvalidParams(String),

    #[error("this action needs a target")]
    TargetRequired,

    #[error("{0}")]
    Failed(String),

    #[error("action id {0:?} is invalid")]
    InvalidId(&'static str),

    #[error("action {0} is already registered")]
    Duplicate(&'static str),
}

impl ActionError {
    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed(reason.into())
    }
}
