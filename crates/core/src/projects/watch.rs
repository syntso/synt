use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError, Weak};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::Project;
use super::project::RESOURCES;
use crate::CoreError;

pub(super) fn watch(
    project: &Arc<Mutex<Project>>,
    path: &Path,
) -> Result<RecommendedWatcher, CoreError> {
    let reloaded = Arc::downgrade(project);
    let cannot_watch = |error: notify::Error| {
        CoreError::Project(format!("cannot watch {}: {error}", path.display()))
    };

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        changed(&reloaded, event);
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

fn changed(reloaded: &Weak<Mutex<Project>>, event: notify::Result<Event>) {
    let Some(project) = reloaded.upgrade() else {
        return;
    };

    let mut project = project.lock().unwrap_or_else(PoisonError::into_inner);

    match event {
        Ok(event) if matches!(event.kind, EventKind::Access(_)) => return,
        Ok(_) => {}
        Err(error) => eprintln!("synt-core: project {}: watching: {error}", project.slug()),
    }

    for change in project.rescan() {
        eprintln!("synt-core: project {}: {change}", project.slug());
    }
}
