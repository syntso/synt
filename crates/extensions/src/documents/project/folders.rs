use super::ProjectFolder;

pub trait ProjectFolders: Send + Sync + 'static {
    fn folder(&self, project: &str) -> Result<ProjectFolder, String>;
}
