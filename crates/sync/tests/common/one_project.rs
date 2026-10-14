use std::path::{Path, PathBuf};

use synt_sync::{ProjectFolder, ProjectFolders};

pub const PROJECT_ID: &str = "4f2c9a7e1b3d5f60718293a4b5c6d7e8";
pub const PROJECT_SLUG: &str = "orders";

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

impl ProjectFolders for OneProject {
    fn folder(&self, project: &str) -> Result<ProjectFolder, String> {
        if project != PROJECT_ID && project != PROJECT_SLUG {
            return Err(format!("no open project is called {project}"));
        }

        Ok(ProjectFolder {
            id: PROJECT_ID.to_owned(),
            path: self.path.clone(),
        })
    }
}
