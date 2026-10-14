#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Recorded {
    pub(crate) edits: usize,
    pub(crate) problems: Vec<String>,
}
