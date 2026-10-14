use synt_sync::{ProjectFolder, ProjectFolders};

use super::Projects;

impl ProjectFolders for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, String> {
        let (id, path) = self.location(project).map_err(|error| error.to_string())?;

        Ok(ProjectFolder { id, path })
    }
}
