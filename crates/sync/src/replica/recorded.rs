#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Recorded {
    pub(crate) edits: usize,
    pub(crate) problems: Vec<String>,
}

impl Recorded {
    pub(super) fn new(problems: Vec<String>) -> Self {
        Self { edits: 0, problems }
    }
}
