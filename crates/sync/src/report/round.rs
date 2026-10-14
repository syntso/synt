use serde::Serialize;

use super::Applied;
use crate::SyncError;
use crate::clock::unix_millis;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RoundReport {
    pub at: u64,
    pub recorded: usize,
    pub pulled: usize,
    pub held: usize,
    pub refused: usize,
    pub merged: usize,
    pub pushed: usize,
    pub applied: Vec<Applied>,
    pub problems: Vec<String>,
    pub error: Option<String>,
}

impl RoundReport {
    pub(crate) fn starting() -> Self {
        Self {
            at: unix_millis(),
            ..Self::default()
        }
    }

    pub(crate) fn failed(error: &SyncError) -> Self {
        Self {
            error: Some(error.to_string()),
            ..Self::starting()
        }
    }
}
