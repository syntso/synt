use serde::Serialize;

use super::redis_command::RedisCommand;
use super::sql::SqlText;
use crate::{Effect, Engine};

pub(super) const UNPARSABLE: &str = "unparsable";
const SEVERAL: &str = "several statements";
const EMPTY: &str = "empty";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Classification {
    pub effect: Effect,
    pub kind: String,
}

impl Classification {
    pub fn of_sql(engine: Engine, text: &str) -> Self {
        SqlText::new(engine, text).classification()
    }

    pub fn of_command<S: AsRef<str>>(words: &[S]) -> Self {
        RedisCommand::parse(words).map_or_else(Self::empty, |command| command.classification())
    }

    pub fn is_read(&self) -> bool {
        self.effect == Effect::Read
    }

    pub fn is_one_statement(&self) -> bool {
        ![UNPARSABLE, SEVERAL, EMPTY].contains(&self.kind.as_str())
    }

    pub(crate) fn new(effect: Effect, kind: impl Into<String>) -> Self {
        Self {
            effect,
            kind: kind.into(),
        }
    }

    pub(super) fn unparsable() -> Self {
        Self::new(Effect::Privileged, UNPARSABLE)
    }

    pub(super) fn several() -> Self {
        Self::new(Effect::Privileged, SEVERAL)
    }

    pub(super) fn empty() -> Self {
        Self::new(Effect::Privileged, EMPTY)
    }
}

#[cfg(test)]
mod tests {
    use super::Classification;
    use crate::Engine;

    #[test]
    fn only_a_single_parsed_statement_is_one_statement() {
        assert!(Classification::of_sql(Engine::Postgres, "SELECT 1").is_one_statement());

        for text in ["SELECT 1; SELECT 2", "SELEKT everything", " ; "] {
            assert!(
                !Classification::of_sql(Engine::Postgres, text).is_one_statement(),
                "{text}"
            );
        }
    }
}
