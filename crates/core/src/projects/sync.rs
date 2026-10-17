use synt_sync::{OpenProjects, ProjectFolder, ProjectId, SyncError};

use super::Projects;

impl OpenProjects for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, SyncError> {
        let place = self.place(project)?;

        Ok(ProjectFolder::new(
            ProjectId::parse(&place.id.to_string())?,
            place.path,
        ))
    }
}
