use super::folder::RunbookFolder;
use super::{RunbookEntry, RunbookListing};
use crate::CoreError;
use crate::projects::{Projects, check_name};
use crate::runbooks::document::Runbook;

#[derive(Debug, Clone)]
pub(crate) struct RunbookStore {
    projects: Projects,
}

impl RunbookStore {
    pub(crate) fn new(projects: Projects) -> Self {
        Self { projects }
    }

    pub(crate) fn reference(&self, project: &str, name: &str) -> Result<String, CoreError> {
        check_name(name).map_err(CoreError::Runbook)?;
        let folder = self.folder(project)?;

        Ok(folder.reference(name))
    }

    pub(crate) fn list(&self, project: &str) -> Result<RunbookListing, CoreError> {
        self.folder(project)?.listing()
    }

    pub(crate) fn get(&self, project: &str, name: &str) -> Result<(String, Runbook), CoreError> {
        check_name(name).map_err(CoreError::Runbook)?;
        let folder = self.folder(project)?;

        let runbook = folder.read(name).map_err(|error| {
            CoreError::Runbook(format!(
                "runbook {name} of project {}: {error}",
                folder.slug()
            ))
        })?;

        Ok((folder.reference(name), runbook))
    }

    pub(crate) fn put(
        &self,
        project: &str,
        name: &str,
        runbook: Runbook,
        admitted: Option<&str>,
    ) -> Result<RunbookEntry, CoreError> {
        check_name(name).map_err(CoreError::Runbook)?;
        runbook.check().map_err(CoreError::Runbook)?;

        let folder = self.folder(project)?;
        folder.admit(name, admitted)?;

        let text = runbook
            .to_toml()
            .map_err(|error| CoreError::Runbook(format!("cannot write {name}: {error}")))?;

        folder
            .write(name, &text)
            .map_err(|error| CoreError::Project(format!("cannot write runbook {name}: {error}")))?;

        Ok(RunbookEntry {
            name: name.to_owned(),
            runbook,
        })
    }

    fn folder(&self, project: &str) -> Result<RunbookFolder, CoreError> {
        let place = self.projects.place(project)?;

        Ok(RunbookFolder::new(place))
    }
}
