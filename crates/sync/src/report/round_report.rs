use serde::Serialize;

use super::Applied;

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
