use crate::report::Applied;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) merged: bool,
    pub(crate) applied: Option<Applied>,
}
