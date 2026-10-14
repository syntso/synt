use crate::report::Applied;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settled {
    pub(crate) applied: Vec<Applied>,
    pub(crate) merged: usize,
}
