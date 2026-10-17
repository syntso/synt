use std::error::Error;

use synt_extensions::{ProjectFolder, ProjectFolders};

use crate::projects::Projects;

impl ProjectFolders for Projects {
    fn folder(&self, project: &str) -> Result<ProjectFolder, Box<dyn Error + Send + Sync>> {
        let place = self.place(project)?;

        Ok(ProjectFolder::new(place.id.to_string(), place.path))
    }
}
