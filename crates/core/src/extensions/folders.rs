use std::error::Error;

use synt_extensions::{ProjectFolder, ProjectFolders};

use crate::projects::Projects;

pub struct CoreFolders {
    projects: Projects,
}

impl CoreFolders {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl ProjectFolders for CoreFolders {
    fn folder(&self, project: &str) -> Result<ProjectFolder, Box<dyn Error + Send + Sync>> {
        let place = self.projects.place(project)?;

        Ok(ProjectFolder::new(place.id.to_string(), place.path))
    }
}
