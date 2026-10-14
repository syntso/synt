use synt_actions::ActionError;
use synt_db::DbError;

pub(super) fn failure(error: DbError) -> ActionError {
    match error {
        DbError::InvalidInput(reason) => ActionError::InvalidParams(reason),
        DbError::Server {
            message,
            code: Some(code),
        } => ActionError::Failed(format!("the server refused: {message} ({code})")),
        other => ActionError::Failed(other.to_string()),
    }
}
