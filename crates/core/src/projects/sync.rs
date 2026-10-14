use synt_sync::{OpenProjects, ProjectFolder, ProjectId, SyncError};

use super::Projects;

impl OpenProjects for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, SyncError> {
        let (id, path) = self.location(project)?;

        Ok(ProjectFolder::new(ProjectId::parse(&id)?, path))
    }
}
