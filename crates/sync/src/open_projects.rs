use crate::{ProjectFolder, ProjectSlug, SyncError};

pub trait OpenProjects: Send + Sync {
    fn folder(&self, project: &ProjectSlug) -> Result<ProjectFolder, SyncError>;
}
