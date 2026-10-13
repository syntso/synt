use std::fmt;

use serde::Serialize;

use super::Damage;
use crate::model::Seq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Verification {
    Intact { checked: u64 },
    Broken { at: Seq, damage: Damage },
    Truncated { last: Seq, head: Seq },
}

impl Verification {
    pub fn is_intact(&self) -> bool {
        matches!(self, Self::Intact { .. })
    }
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intact { checked } => write!(f, "intact, {checked} events checked"),
            Self::Broken {
                at,
                damage: Damage::Altered,
            } => write!(f, "event {at} was altered"),
            Self::Broken {
                at,
                damage: Damage::Missing,
            } => write!(f, "event {at} is missing"),
            Self::Truncated { last, head } => {
                write!(
                    f,
                    "events after {last} were removed; the journal had reached {head}"
                )
            }
        }
    }
}
