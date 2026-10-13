use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use notify::RecommendedWatcher;

use super::Project;
use super::watch::watch;
use crate::CoreError;

pub(crate) struct OpenProject {
    project: Arc<Mutex<Project>>,
    _watcher: RecommendedWatcher,
}

impl OpenProject {
    pub(crate) fn start(project: Project) -> Result<Self, CoreError> {
        let path = project.path().to_owned();
        let project = Arc::new(Mutex::new(project));
        let watcher = watch(&project, &path)?;

        Ok(Self {
            project,
            _watcher: watcher,
        })
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Project> {
        self.project.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
