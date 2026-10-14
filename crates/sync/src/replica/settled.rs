use super::Outcome;
use crate::report::Applied;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settled {
    pub(crate) applied: Vec<Applied>,
    pub(crate) merged: usize,
    pub(crate) problems: Vec<String>,
}

impl Settled {
    pub(crate) fn add(&mut self, outcome: Outcome) {
        self.merged += usize::from(outcome.merged);
        self.applied.extend(outcome.applied);
    }
}
