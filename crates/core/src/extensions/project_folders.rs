use synt_extensions::{ProjectFolder, ProjectFolders};

use crate::projects::Projects;

impl ProjectFolders for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, String> {
        let (id, path) = Projects::folder(self, project).map_err(|error| error.to_string())?;

        Ok(ProjectFolder { id, path })
    }
}
