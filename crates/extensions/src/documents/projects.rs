mod folder;

use std::error::Error;

pub use folder::ProjectFolder;

pub trait ProjectFolders: Send + Sync + 'static {
    fn folder(&self, project: &str) -> Result<ProjectFolder, Box<dyn Error + Send + Sync>>;
}
