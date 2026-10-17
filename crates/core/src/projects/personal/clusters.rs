use crate::projects::{Problem, ResourceEntry};

#[derive(Debug, Default)]
pub(super) struct Clusters {
    pub(super) entries: Vec<ResourceEntry>,
    pub(super) problems: Vec<Problem>,
}
