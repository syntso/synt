use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use synt_platform::Lock;
use tokio::sync::broadcast;

use super::{LoadedProject, ProjectChange, ResourceFolder};
use crate::CoreError;
use crate::log::log;

pub(crate) struct OpenProject {
    folder: Arc<Mutex<LoadedProject>>,
    _watcher: RecommendedWatcher,
}

impl OpenProject {
    pub(crate) fn start(
        folder: LoadedProject,
        changes: broadcast::Sender<ProjectChange>,
    ) -> Result<Self, CoreError> {
        let path = folder.path().to_owned();
        let folder = Arc::new(Mutex::new(folder));
        let watcher = Self::watch(&folder, &path, changes)?;

        Ok(Self {
            folder,
            _watcher: watcher,
        })
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, LoadedProject> {
        self.folder.locked()
    }

    fn watch(
        folder: &Arc<Mutex<LoadedProject>>,
        path: &Path,
        changes: broadcast::Sender<ProjectChange>,
    ) -> Result<RecommendedWatcher, CoreError> {
        let owner = Arc::downgrade(folder);
        let cannot_watch = |source: notify::Error| CoreError::Watch {
            path: path.to_owned(),
            source,
        };

        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            Self::changed(&owner, event, &changes);
        })
        .map_err(cannot_watch)?;

        let mut paths = watcher.paths_mut();
        paths
            .add(path, RecursiveMode::NonRecursive)
            .map_err(cannot_watch)?;
        paths
            .add(ResourceFolder::of(path).path(), RecursiveMode::NonRecursive)
            .map_err(cannot_watch)?;
        paths.commit().map_err(cannot_watch)?;

        Ok(watcher)
    }

    fn changed(
        watched: &Weak<Mutex<LoadedProject>>,
        event: notify::Result<Event>,
        changes: &broadcast::Sender<ProjectChange>,
    ) {
        let Some(folder) = watched.upgrade() else {
            return;
        };

        let mut folder = folder.locked();

        match event {
            Ok(event) if matches!(event.kind, EventKind::Access(_)) => return,
            Ok(_) => {}
            Err(error) => log!("project {}: watching: {error}", folder.slug()),
        }

        for change in folder.rescan() {
            log!("project {}: {change}", folder.slug());

            let _ = changes.send(ProjectChange {
                project: folder.id().to_string(),
                change,
            });
        }
    }
}
