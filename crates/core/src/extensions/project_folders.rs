use std::error::Error;

use synt_extensions::{ProjectFolder, ProjectFolders};

use crate::projects::Projects;

impl ProjectFolders for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, Box<dyn Error + Send + Sync>> {
        let (id, path) = Projects::folder(self, project)?;

        Ok(ProjectFolder::new(id, path))
    }
}
