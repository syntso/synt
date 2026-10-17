use synt_sync::{OpenProjects, ProjectFolder, SyncError};

use super::{ProjectSlug, Projects};

impl OpenProjects for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, SyncError> {
        let slug = ProjectSlug::named(project)?;

        Ok(self.place(&slug)?)
    }
}
