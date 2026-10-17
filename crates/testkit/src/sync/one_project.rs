use std::path::{Path, PathBuf};

use synt_sync::{OpenProjects, ProjectFolder, ProjectId, ProjectSlug, SyncError};

pub const PROJECT_ID: &str = "4f2c9a7e1b3d5f60718293a4b5c6d7e8";
pub const PROJECT_SLUG: &str = "orders";

pub fn project_slug() -> Result<ProjectSlug, SyncError> {
    ProjectSlug::named(PROJECT_SLUG)
}

pub struct OneProject {
    path: PathBuf,
}

impl OneProject {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
}

impl OpenProjects for OneProject {
    fn folder(&self, project: &ProjectSlug) -> Result<ProjectFolder, SyncError> {
        if project.as_str() != PROJECT_ID && project.as_str() != PROJECT_SLUG {
            return Err(SyncError::Projects(
                format!("no open project is called {project}").into(),
            ));
        }

        Ok(ProjectFolder::new(
            ProjectId::parse(PROJECT_ID)?,
            self.path.clone(),
        ))
    }
}
