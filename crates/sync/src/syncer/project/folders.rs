use super::ProjectFolder;

pub trait ProjectFolders: Send + Sync {
    fn folder(&self, project: &str) -> Result<ProjectFolder, String>;
}
