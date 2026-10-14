mod outcome;
mod recorded;
mod scan;
mod settled;
mod target;

pub(crate) use recorded::Recorded;
pub(crate) use settled::Settled;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::clock::unix_millis;
use crate::document::{DocumentPath, Draft, Revision};
use crate::history::{History, Origin, StoredDocument};
use crate::id::{MemberId, ProjectId, RevisionId};
use crate::merge::{DocumentGraph, DocumentMerger, ThreeWay};
use crate::report::RevisionInfo;
use crate::seal::Naming;
use crate::{ProjectFolder, SyncError};
use outcome::Outcome;
use scan::Scan;
use target::Target;

pub(crate) struct Replica {
    history: History,
    folder: ProjectFolder,
    naming: Naming,
    me: MemberId,
    merger: Option<Arc<dyn DocumentMerger>>,
}

impl Replica {
    pub(crate) fn new(
        history: History,
        folder: ProjectFolder,
        naming: Naming,
        me: MemberId,
        merger: Option<Arc<dyn DocumentMerger>>,
    ) -> Self {
        Self {
            history,
            folder,
            naming,
            me,
            merger,
        }
    }

    pub(crate) fn project(&self) -> &ProjectId {
        self.folder.id()
    }

    pub(crate) fn record_edits(&self) -> Result<Recorded, SyncError> {
        self.folder.check()?;

        let scan = self.scan()?;
        scan.refuse_removing_everything()?;

        Ok(scan.record(|edit| self.record(edit)))
    }

    pub(crate) fn accept(&self, revision: &Revision) -> Result<bool, SyncError> {
        self.history
            .insert(self.project(), revision, Origin::Pulled)
    }

    pub(crate) fn settle(&self) -> Result<Settled, SyncError> {
        let mut settled = Settled::default();

        for stored in self.history.documents(self.project())? {
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

        let graph = self.graph_of(path)?;
        let chosen = graph.get(chosen).ok_or_else(|| SyncError::NotInHistory {
            revision: chosen.clone(),
            path: path.clone(),
        })?;

        if !graph.has_conflict() {
            return Err(SyncError::NoConflict(path.clone()));
        }

        let heads = graph.heads().iter().map(|head| head.id.clone()).collect();
        let resolution = self.authored(path, heads, chosen.body.clone());

        self.history
            .insert(self.project(), &resolution, Origin::Local)?;
        self.settle()?;

        Ok(resolution)
    }

    pub(crate) fn history_of(&self, path: &DocumentPath) -> Result<Vec<RevisionInfo>, SyncError> {
        let graph = self.graph_of(path)?;
        let on_disk = self.stored(path)?.and_then(|stored| stored.on_disk);
        let heads: BTreeSet<&RevisionId> = graph.heads().iter().map(|head| &head.id).collect();

        let infos = graph
            .revisions()
            .rev()
            .map(|revision| {
                let head = heads.contains(&revision.id);
                let on_disk = on_disk.as_ref() == Some(&revision.id);

                revision.info(head, on_disk)
            })
            .collect();

        Ok(infos)
    }

    fn scan(&self) -> Result<Scan, SyncError> {
        let known = self.known_documents()?;
        let mut paths: BTreeSet<DocumentPath> = self.folder.paths()?;
        paths.extend(known.keys().cloned());

        let mut scan = Scan::default();

        for path in paths {
            if let Err(error) = self.look_at(&path, known.get(&path), &mut scan) {
                scan.failed(&path, &error);
            }
        }

        Ok(scan)
    }

    fn look_at(
        &self,
        path: &DocumentPath,
        stored: Option<&StoredDocument>,
        scan: &mut Scan,
    ) -> Result<(), SyncError> {
        let current = self.folder.read(path)?;
        let on_disk = self.on_disk(stored)?;

        let unchanged = match &on_disk {
            Some(revision) => revision.body == current,
            None => current.is_none(),
        };

        let edit = (!unchanged).then(|| {
            let parents = on_disk.iter().map(|revision| revision.id.clone()).collect();

            self.authored(path, parents, current)
        });
        scan.found(path, on_disk.as_ref(), edit);

        Ok(())
    }

    fn authored(
        &self,
        path: &DocumentPath,
        parents: Vec<RevisionId>,
        body: Option<Vec<u8>>,
    ) -> Revision {
        let draft = Draft {
            path: path.clone(),
            parents,
            at: unix_millis(),
            body,
            conflicts: Vec::new(),
            author: Some(self.me.clone()),
        };

        Revision::new(&self.naming, draft)
    }

    fn record(&self, edit: &Revision) -> Result<(), SyncError> {
        self.history.insert(self.project(), edit, Origin::Local)?;
        self.history.set_on_disk(self.project(), edit)
    }

    fn settle_document(&self, stored: &StoredDocument) -> Result<Outcome, SyncError> {
        let graph = self.history.graph(self.project(), &stored.document)?;

        let Some(target) = self.target(&graph)? else {
            return Ok(Outcome::default());
        };

        let moved = stored.on_disk.as_ref() != Some(&target.revision.id);
        let applied = moved && self.apply(stored, &graph, &target.revision)?;

        Ok(Outcome {
            merged: target.merged,
            applied: applied.then(|| target.revision.applied()),
        })
    }

    fn target(&self, graph: &DocumentGraph) -> Result<Option<Target>, SyncError> {
        let heads = graph.heads();

        if let [head] = heads.as_slice() {
            return Ok(Some(Target::head((*head).clone())));
        }

        let Some(first) = heads.first() else {
            return Ok(None);
        };

        let base = graph
            .merge_base(&heads)
            .and_then(|base| base.body.as_deref());
        let three_way = ThreeWay::new(first.path.kind(), base, self.merger.as_deref());

        let Some(merged) = three_way.heads(&heads) else {
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
        self.history.insert(self.project(), &merge, Origin::Local)?;

        Ok(Some(Target::merge(merge)))
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
        let current = self.folder.read(&target.path)?;

        if current.as_deref() != expected {
            return Ok(false);
        }

        let changes = current != target.body;
        match &target.body {
            Some(body) if changes => self.folder.write(&target.path, body)?,
            None if changes => self.folder.remove(&target.path)?,
            _ => {}
        }
        self.history.set_on_disk(self.project(), target)?;

        Ok(changes)
    }

    fn graph_of(&self, path: &DocumentPath) -> Result<DocumentGraph, SyncError> {
        let document = self.naming.document_id(path);

        self.history.graph(self.project(), &document)
    }

    fn known_documents(&self) -> Result<BTreeMap<DocumentPath, StoredDocument>, SyncError> {
        let documents = self.history.documents(self.project())?;

        Ok(documents
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

        let graph = self.history.graph(self.project(), document)?;

        Ok(graph.get(on_disk).cloned())
    }
}
