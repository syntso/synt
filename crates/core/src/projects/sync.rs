use synt_sync::{OpenProjects, ProjectFolder, SyncError};

use super::{ProjectSlug, Projects};

impl OpenProjects for Projects {
    fn folder(&self, project: &ProjectSlug) -> Result<ProjectFolder, SyncError> {
        Ok(self.place(project)?)
    }
}
