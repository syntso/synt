use crate::report::Applied;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Outcome {
    pub(super) merged: bool,
    pub(super) applied: Option<Applied>,
}
