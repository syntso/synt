use std::path::Path;
use std::sync::{Arc, Mutex, Weak};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::broadcast;

use super::project::RESOURCES;
use super::{Project, ProjectChange};
use crate::CoreError;
use crate::locked::locked;
use crate::log::log;

pub(super) fn watch(
    project: &Arc<Mutex<Project>>,
    path: &Path,
    changes: broadcast::Sender<ProjectChange>,
) -> Result<RecommendedWatcher, CoreError> {
    let reloaded = Arc::downgrade(project);
    let cannot_watch = |error: notify::Error| {
        CoreError::Project(format!("cannot watch {}: {error}", path.display()))
    };

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        changed(&reloaded, event, &changes);
    })
    .map_err(cannot_watch)?;

    let mut paths = watcher.paths_mut();
    paths
        .add(path, RecursiveMode::NonRecursive)
        .map_err(cannot_watch)?;
    paths
        .add(&path.join(RESOURCES), RecursiveMode::NonRecursive)
        .map_err(cannot_watch)?;
    paths.commit().map_err(cannot_watch)?;

    Ok(watcher)
}

fn changed(
    reloaded: &Weak<Mutex<Project>>,
    event: notify::Result<Event>,
    changes: &broadcast::Sender<ProjectChange>,
) {
    let Some(project) = reloaded.upgrade() else {
        return;
    };

    let mut project = locked(&project);

    match event {
        Ok(event) if matches!(event.kind, EventKind::Access(_)) => return,
        Ok(_) => {}
        Err(error) => log!("project {}: watching: {error}", project.slug()),
    }

    for change in project.rescan() {
        log!("project {}: {change}", project.slug());

        let _ = changes.send(ProjectChange {
            project: project.id().to_owned(),
            change,
        });
    }
}
