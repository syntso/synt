use crate::SyncError;
use crate::document::{DocumentPath, Revision};
use crate::history::History;
use crate::id::ProjectId;
use crate::report::ConflictInfo;

pub(crate) fn conflicts(
    history: &History,
    project: &ProjectId,
) -> Result<Vec<ConflictInfo>, SyncError> {
    let mut conflicts = Vec::new();

    for stored in history.documents(project)? {
        let graph = history.graph(project, &stored.document)?;
        let heads = graph.heads();

        if is_conflicted(&heads) {
            conflicts.push(conflict(&stored.path, &heads));
        }
    }

    Ok(conflicts)
}

pub(crate) fn is_conflicted(heads: &[&Revision]) -> bool {
    match heads {
        [head] => !head.conflicts.is_empty(),
        _ => heads.len() > 1,
    }
}

fn conflict(path: &DocumentPath, heads: &[&Revision]) -> ConflictInfo {
    let path = path.get().to_owned();

    if let [merge] = heads {
        return ConflictInfo {
            path,
            merged: Some(merge.id.clone()),
            candidates: merge.parents.clone(),
            fields: merge.conflicts.clone(),
        };
    }

    ConflictInfo {
        path,
        merged: None,
        candidates: heads.iter().map(|head| head.id.clone()).collect(),
        fields: Vec::new(),
    }
}
