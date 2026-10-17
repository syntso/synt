use serde::Serialize;
use synt_journal::Timestamp;

use super::Applied;
use crate::SyncError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RoundReport {
    pub at: Timestamp,
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
    pub(crate) fn starting(at: Timestamp) -> Self {
        Self {
            at,
            ..Self::default()
        }
    }

    pub(crate) fn failed(error: &SyncError, at: Timestamp) -> Self {
        Self {
            error: Some(error.to_string()),
            ..Self::starting(at)
        }
    }
}
