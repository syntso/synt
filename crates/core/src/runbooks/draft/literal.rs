use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Literal {
    pub(super) text: String,
    pub(super) hint: String,
    pub(super) is_target: bool,
    pub(super) steps: BTreeSet<usize>,
}

impl Literal {
    pub(super) fn repeats(&self) -> bool {
        self.steps.len() > 1
    }
}
