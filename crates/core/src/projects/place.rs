use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectPlace {
    pub(crate) id: String,
    pub(crate) slug: String,
    pub(crate) path: PathBuf,
}
