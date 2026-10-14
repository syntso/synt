use std::sync::{Arc, Mutex, MutexGuard};

use notify::RecommendedWatcher;

use tokio::sync::broadcast;

use super::watch::watch;
use super::{Project, ProjectChange};
use crate::CoreError;
use crate::locked::locked;

pub(crate) struct OpenProject {
    project: Arc<Mutex<Project>>,
    _watcher: RecommendedWatcher,
}

impl OpenProject {
    pub(crate) fn start(
        project: Project,
        changes: broadcast::Sender<ProjectChange>,
    ) -> Result<Self, CoreError> {
        let path = project.path().to_owned();
        let project = Arc::new(Mutex::new(project));
        let watcher = watch(&project, &path, changes)?;

        Ok(Self {
            project,
            _watcher: watcher,
        })
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Project> {
        locked(&self.project)
    }
}
