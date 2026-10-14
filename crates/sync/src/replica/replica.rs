use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::conflicts::is_conflicted;
use super::{Outcome, Recorded, Settled};
use crate::SyncError;
use crate::clock::unix_millis;
use crate::document::{DocumentPath, Draft, ProjectFiles, Revision};
use crate::history::{History, StoredDocument};
use crate::id::{MemberId, ProjectId, RevisionId};
use crate::merge::{DocumentGraph, DocumentMerger, merge_heads};
use crate::report::{Applied, RevisionInfo};
use crate::seal::Naming;

pub(crate) struct Replica {
    history: History,
    project: ProjectId,
    files: ProjectFiles,
    naming: Naming,
    me: MemberId,
    merger: Option<Arc<dyn DocumentMerger>>,
}

impl Replica {
    pub(crate) fn new(
        history: History,
        project: ProjectId,
        files: ProjectFiles,
        naming: Naming,
        me: MemberId,
    ) -> Self {
        Self {
            history,
            project,
            files,
            naming,
            me,
            merger: None,
        }
    }

    #[must_use]
    pub(crate) fn with_merger(mut self, merger: Option<Arc<dyn DocumentMerger>>) -> Self {
        self.merger = merger;
        self
    }

    pub(crate) fn record_edits(&self) -> Result<Recorded, SyncError> {
        let known = self.known_documents()?;
        let mut paths: BTreeSet<DocumentPath> = self.files.paths()?;
        paths.extend(known.keys().cloned());

        let mut recorded = Recorded::default();

        for path in paths {
            match self.record_edit(&path, known.get(&path)) {
                Ok(true) => recorded.edits += 1,
                Ok(false) => {}
                Err(error) => recorded.problems.push(format!("{path}: {error}")),
            }
        }

        Ok(recorded)
    }

    pub(crate) fn accept(&self, revision: &Revision) -> Result<bool, SyncError> {
        self.history.insert(&self.project, revision, true)
    }

    pub(crate) fn settle(&self) -> Result<Settled, SyncError> {
        let mut settled = Settled::default();

        for stored in self.history.documents(&self.project)? {
            match self.settle_document(&stored) {
                Ok(outcome) => settled.add(outcome),
                Err(error) => settled.problems.push(format!("{}: {error}", stored.path)),
            }
        }

        Ok(settled)
    }

    pub(crate) fn resolve(
        &self,
        path: &DocumentPath,
        chosen: &RevisionId,
    ) -> Result<Revision, SyncError> {
        self.record_edits()?;

        let document = self.naming.document_id(path);
        let graph = self.history.graph(&self.project, &document)?;
        let chosen = graph.get(chosen).ok_or_else(|| {
            SyncError::invalid(format!("revision {chosen} is not in the history of {path}"))
        })?;

        let heads = graph.heads();
        if !is_conflicted(&heads) {
            return Err(SyncError::invalid(format!(
                "{path} has no conflict to resolve"
            )));
        }

        let resolution = Revision::new(
            &self.naming,
            Draft {
                path: path.clone(),
                parents: heads.iter().map(|head| head.id.clone()).collect(),
                at: unix_millis(),
                body: chosen.body.clone(),
                conflicts: Vec::new(),
                author: Some(self.me.clone()),
            },
        );
        self.history.insert(&self.project, &resolution, false)?;
        self.settle()?;

        Ok(resolution)
    }

    pub(crate) fn history_of(&self, path: &DocumentPath) -> Result<Vec<RevisionInfo>, SyncError> {
        let document = self.naming.document_id(path);
        let graph = self.history.graph(&self.project, &document)?;
        let on_disk = self.stored(path)?.and_then(|stored| stored.on_disk);
        let heads: BTreeSet<&RevisionId> = graph.heads().iter().map(|head| &head.id).collect();

        Ok(graph
            .revisions()
            .rev()
            .map(|revision| RevisionInfo {
                revision: revision.id.clone(),
                parents: revision.parents.clone(),
                at: revision.at,
                author: revision.author.clone(),
                removed: revision.is_removal(),
                bytes: revision.body.as_ref().map_or(0, Vec::len),
                conflicts: revision.conflicts.clone(),
                head: heads.contains(&revision.id),
                on_disk: on_disk.as_ref() == Some(&revision.id),
            })
            .collect())
    }

    fn record_edit(
        &self,
        path: &DocumentPath,
        stored: Option<&StoredDocument>,
    ) -> Result<bool, SyncError> {
        let current = self.files.read(path)?;
        let on_disk = self.on_disk(stored)?;

        let unchanged = match &on_disk {
            Some(revision) => revision.body == current,
            None => current.is_none(),
        };
        if unchanged {
            return Ok(false);
        }

        let edit = Revision::new(
            &self.naming,
            Draft {
                path: path.clone(),
                parents: on_disk.map(|revision| revision.id).into_iter().collect(),
                at: unix_millis(),
                body: current,
                conflicts: Vec::new(),
                author: Some(self.me.clone()),
            },
        );

        self.history.insert(&self.project, &edit, false)?;
        self.history.set_on_disk(&self.project, &edit)?;

        Ok(true)
    }

    fn settle_document(&self, stored: &StoredDocument) -> Result<Outcome, SyncError> {
        let graph = self.history.graph(&self.project, &stored.document)?;

        let Some((target, merged)) = self.target(&graph)? else {
            return Ok(Outcome::default());
        };

        let applied =
            stored.on_disk.as_ref() != Some(&target.id) && self.apply(stored, &graph, &target)?;

        Ok(Outcome {
            merged,
            applied: applied.then(|| applied_change(&target)),
        })
    }

    fn target(&self, graph: &DocumentGraph) -> Result<Option<(Revision, bool)>, SyncError> {
        let heads = graph.heads();

        if let [head] = heads.as_slice() {
            return Ok(Some(((*head).clone(), false)));
        }

        let Some(first) = heads.first() else {
            return Ok(None);
        };

        let base = graph
            .merge_base(&heads)
            .and_then(|base| base.body.as_deref());
        let extensions = self.merger.as_deref();

        let Some(merged) = merge_heads(first.path.kind(), base, &heads, extensions) else {
            return Ok(None);
        };

        let merge = Revision::new(
            &self.naming,
            Draft {
                path: first.path.clone(),
                parents: heads.iter().map(|head| head.id.clone()).collect(),
                at: heads.iter().map(|head| head.at).max().unwrap_or_default(),
                body: merged.body,
                conflicts: merged.conflicts,
                author: None,
            },
        );
        self.history.insert(&self.project, &merge, false)?;

        Ok(Some((merge, true)))
    }

    fn apply(
        &self,
        stored: &StoredDocument,
        graph: &DocumentGraph,
        target: &Revision,
    ) -> Result<bool, SyncError> {
        let expected = stored
            .on_disk
            .as_ref()
            .and_then(|id| graph.get(id))
            .and_then(|revision| revision.body.as_deref());
        let current = self.files.read(&target.path)?;

        if current.as_deref() != expected {
            return Ok(false);
        }

        let changes = current != target.body;
        match &target.body {
            Some(body) if changes => self.files.write(&target.path, body)?,
            None if changes => self.files.remove(&target.path)?,
            _ => {}
        }
        self.history.set_on_disk(&self.project, target)?;

        Ok(changes)
    }

    fn known_documents(&self) -> Result<BTreeMap<DocumentPath, StoredDocument>, SyncError> {
        Ok(self
            .history
            .documents(&self.project)?
            .into_iter()
            .map(|stored| (stored.path.clone(), stored))
            .collect())
    }

    fn stored(&self, path: &DocumentPath) -> Result<Option<StoredDocument>, SyncError> {
        Ok(self.known_documents()?.remove(path))
    }

    fn on_disk(&self, stored: Option<&StoredDocument>) -> Result<Option<Revision>, SyncError> {
        let Some(StoredDocument {
            document,
            on_disk: Some(on_disk),
            ..
        }) = stored
        else {
            return Ok(None);
        };

        let graph = self.history.graph(&self.project, document)?;

        Ok(graph.get(on_disk).cloned())
    }
}

fn applied_change(revision: &Revision) -> Applied {
    Applied {
        path: revision.path.get().to_owned(),
        revision: revision.id.clone(),
        author: revision.author.clone(),
        removed: revision.is_removal(),
        conflicts: revision.conflicts.len(),
    }
}
