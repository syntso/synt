use synt_sync::{OpenProjects, ProjectFolder, SyncError};

use super::Projects;

impl OpenProjects for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, SyncError> {
        let place = self.place(project)?;

        Ok(ProjectFolder::new(place.id, place.path))
    }
}
