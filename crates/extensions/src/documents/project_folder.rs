use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFolder {
    pub id: String,
    pub path: PathBuf,
}
