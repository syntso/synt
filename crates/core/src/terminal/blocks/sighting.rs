use super::Mark;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sighting {
    pub(crate) mark: Mark,
    pub(crate) start: u64,
    pub(crate) end: u64,
}
