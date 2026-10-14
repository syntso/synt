use synt_sync::{ProjectFolder, ProjectFolders};

use super::Projects;

impl ProjectFolders for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, String> {
        let place = self.place(project).map_err(|error| error.to_string())?;

        Ok(ProjectFolder {
            id: place.id.to_string(),
            path: place.path,
        })
    }
}
