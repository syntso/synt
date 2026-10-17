use crate::{ProjectFolder, SyncError};

pub trait OpenProjects: Send + Sync {
    fn folder(&self, project: &str) -> Result<ProjectFolder, SyncError>;
}
