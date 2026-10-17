use std::time::Duration;

use synt_actions::ActionError;
use synt_db::{DbError, StatementKind};
use synt_ssh::SshError;
use thiserror::Error;

use crate::CoreError;
use crate::builtin::ssh::failure as ssh_failure;

#[derive(Debug, Error)]
pub(crate) enum DbActionError {
    #[error(transparent)]
    Database(#[from] DbError),

    #[error(transparent)]
    Tunnel(#[from] SshError),

    #[error(transparent)]
    Action(#[from] ActionError),

    #[error("{0}")]
    Target(CoreError),

    #[error("{0}")]
    Resource(String),

    #[error("the target is no longer a database")]
    NotADatabase,

    #[error("the database actions are shutting down")]
    ShuttingDown,

    #[error("no answer within {} s, so the call was stopped", .0.as_secs())]
    Timeout(Duration),

    #[error(
        "{target} is now reached through a root login, which the call was not admitted for; \
         run it again so it is asked as privileged"
    )]
    RootLogin { target: String },

    #[error(
        "{target} now leads to {reached}, which is not what the call was admitted for; \
         run it again so it is checked for the new place"
    )]
    Moved { target: String, reached: String },

    #[error("give either sql or command")]
    NoStatement,

    #[error("sql is empty")]
    EmptySql,

    #[error("command is empty")]
    EmptyCommand,

    #[error("a redis command carries its values in the command itself, not in params")]
    CommandParams,

    #[error("redis takes a command, such as [\"GET\", \"user:1\"], not sql")]
    SqlForRedis,

    #[error("this database takes sql, not a command")]
    CommandForSql,

    #[error("{field} must be between 1 and {most}")]
    OutOfRange { field: &'static str, most: u64 },

    #[error("redis has no query plans to explain")]
    NoPlans,

    #[error("db.explain takes one statement the parser understands, got {0}")]
    NotOneStatement(StatementKind),

    #[error("give the statement itself; db.explain adds EXPLAIN")]
    AlreadyExplained,
}

impl DbActionError {
    fn is_invalid_params(&self) -> bool {
        matches!(
            self,
            Self::Target(_)
                | Self::NoStatement
                | Self::EmptySql
                | Self::EmptyCommand
                | Self::CommandParams
                | Self::SqlForRedis
                | Self::CommandForSql
                | Self::OutOfRange { .. }
                | Self::NoPlans
                | Self::NotOneStatement(_)
                | Self::AlreadyExplained
        )
    }
}

impl From<DbActionError> for ActionError {
    fn from(error: DbActionError) -> Self {
        if error.is_invalid_params() {
            return Self::InvalidParams(error.to_string());
        }

        match error {
            DbActionError::Action(action) => action,
            DbActionError::Tunnel(ssh) => ssh_failure(ssh),
            DbActionError::Database(DbError::InvalidInput(reason)) => Self::InvalidParams(reason),
            DbActionError::Database(DbError::Server {
                message,
                code: Some(code),
            }) => Self::Failed(format!("the server refused: {message} ({code})")),
            other => Self::Failed(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use synt_actions::ActionError;
    use synt_db::DbError;

    use super::DbActionError;

    fn action(error: DbActionError) -> ActionError {
        error.into()
    }

    #[test]
    fn bad_input_to_the_database_is_a_parameter_error() {
        let error = DbError::InvalidInput("the command is empty".to_owned());

        assert_eq!(
            action(error.into()),
            ActionError::InvalidParams("the command is empty".to_owned())
        );
    }

    #[test]
    fn a_refusal_names_the_server_code() {
        let error = DbError::Server {
            message: "permission denied".to_owned(),
            code: Some("42501".to_owned()),
        };

        assert_eq!(
            action(error.into()),
            ActionError::Failed("the server refused: permission denied (42501)".to_owned())
        );
    }

    #[test]
    fn statement_mistakes_are_parameter_errors() {
        assert_eq!(
            action(DbActionError::CommandForSql),
            ActionError::InvalidParams("this database takes sql, not a command".to_owned())
        );
    }

    #[test]
    fn a_failed_action_passes_through() {
        assert_eq!(
            action(ActionError::TargetRequired.into()),
            ActionError::TargetRequired
        );
    }
}
