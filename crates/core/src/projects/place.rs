use std::path::PathBuf;

use super::{ProjectId, ProjectSlug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectPlace {
    pub(crate) id: ProjectId,
    pub(crate) slug: ProjectSlug,
    pub(crate) path: PathBuf,
}
