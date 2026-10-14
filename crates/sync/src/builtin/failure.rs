use synt_actions::ActionError;

use crate::SyncError;

pub(super) fn failure(error: SyncError) -> ActionError {
    match error {
        SyncError::Invalid(reason) => ActionError::InvalidParams(reason),
        other => ActionError::failed(other.to_string()),
    }
}

pub(super) fn invalid(error: SyncError) -> ActionError {
    match error {
        SyncError::Invalid(reason) => ActionError::InvalidParams(reason),
        other => ActionError::InvalidParams(other.to_string()),
    }
}
