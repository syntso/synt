use serde::Serialize;

use super::{redis_command, sql};
use crate::{Effect, Engine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Classification {
    pub effect: Effect,
    pub kind: String,
}

impl Classification {
    pub fn of_sql(engine: Engine, text: &str) -> Self {
        sql::classify(engine, text)
    }

    pub fn of_command<S: AsRef<str>>(words: &[S]) -> Self {
        redis_command::classify(words)
    }

    pub(crate) fn new(effect: Effect, kind: impl Into<String>) -> Self {
        Self {
            effect,
            kind: kind.into(),
        }
    }

    pub fn is_read(&self) -> bool {
        self.effect == Effect::Read
    }
}
