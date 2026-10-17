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
    fn folder(&self, project: &str) -> Result<ProjectFolder, String> {
        let (id, path) = self
            .projects
            .folder(project)
            .map_err(|error| error.to_string())?;

        Ok(ProjectFolder { id, path })
    }
}
