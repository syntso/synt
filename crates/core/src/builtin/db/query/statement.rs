use serde_json::Value;
use synt_db::{Access, Classification, Database, DbError, Engine, Page, Query, Rows};

use crate::builtin::db::error::DbActionError;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Statement {
    Sql { sql: String, params: Vec<Value> },
    Command { command: Vec<String> },
}

impl Statement {
    pub(super) fn new(
        sql: Option<String>,
        command: Option<Vec<String>>,
        params: Vec<Value>,
    ) -> Result<Self, DbActionError> {
        match (sql, command) {
            (Some(sql), None) if sql.trim().is_empty() => Err(DbActionError::EmptySql),
            (Some(sql), None) => Ok(Self::Sql { sql, params }),
            (None, Some(command)) if command.is_empty() => Err(DbActionError::EmptyCommand),
            (None, Some(_)) if !params.is_empty() => Err(DbActionError::CommandParams),
            (None, Some(command)) => Ok(Self::Command { command }),
            _ => Err(DbActionError::NoStatement),
        }
    }

    pub(crate) fn classification(&self, engine: Engine) -> Classification {
        match self {
            Self::Sql { sql, .. } => Classification::of_sql(engine, sql),
            Self::Command { command } => Classification::of_command(command),
        }
    }

    pub(super) fn check(&self, engine: Engine) -> Result<(), DbActionError> {
        match self {
            Self::Sql { .. } if !engine.speaks_sql() => Err(DbActionError::SqlForRedis),
            Self::Command { .. } if engine.speaks_sql() => Err(DbActionError::CommandForSql),
            Self::Sql { .. } | Self::Command { .. } => Ok(()),
        }
    }

    pub(super) async fn run(
        &self,
        database: &mut Database,
        access: Access,
        page: Page,
    ) -> Result<Rows, DbError> {
        match self {
            Self::Command { command } => database.command(command, page).await,
            Self::Sql { sql, params } => {
                let query = Query {
                    text: sql,
                    params,
                    access,
                    page,
                };

                database.query(&query).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_db::Engine;

    use super::Statement;
    use crate::builtin::db::error::DbActionError;

    fn sql(text: &str) -> Statement {
        Statement::new(Some(text.to_owned()), None, Vec::new()).expect("sql")
    }

    fn command(words: &[&str]) -> Statement {
        let words = words.iter().map(|word| (*word).to_owned()).collect();

        Statement::new(None, Some(words), Vec::new()).expect("command")
    }

    #[test]
    fn exactly_one_of_sql_and_command_is_a_statement() {
        let both = Statement::new(
            Some("SELECT 1".to_owned()),
            Some(vec!["GET".to_owned()]),
            vec![],
        );
        let neither = Statement::new(None, None, vec![]);

        assert!(matches!(both, Err(DbActionError::NoStatement)));
        assert!(matches!(neither, Err(DbActionError::NoStatement)));
    }

    #[test]
    fn empty_statements_are_refused() {
        let blank = Statement::new(Some("  ".to_owned()), None, vec![]);
        let wordless = Statement::new(None, Some(vec![]), vec![]);

        assert!(matches!(blank, Err(DbActionError::EmptySql)));
        assert!(matches!(wordless, Err(DbActionError::EmptyCommand)));
    }

    #[test]
    fn a_command_carries_no_params() {
        let given = Statement::new(None, Some(vec!["GET".to_owned()]), vec![json!(1)]);

        assert!(matches!(given, Err(DbActionError::CommandParams)));
    }

    #[test]
    fn each_engine_takes_its_own_kind_of_statement() {
        assert!(sql("SELECT 1").check(Engine::Postgres).is_ok());
        assert!(command(&["GET", "k"]).check(Engine::Redis).is_ok());
        assert!(matches!(
            sql("SELECT 1").check(Engine::Redis),
            Err(DbActionError::SqlForRedis)
        ));
        assert!(matches!(
            command(&["GET", "k"]).check(Engine::Sqlite),
            Err(DbActionError::CommandForSql)
        ));
    }
}
