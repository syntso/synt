use super::Recorded;
use crate::SyncError;
use crate::document::{DocumentKind, DocumentPath, Revision};

const EVERYTHING_AT_LEAST: usize = 2;

#[derive(Debug, Default)]
pub(super) struct Scan {
    edits: Vec<Revision>,
    problems: Vec<String>,
    live: usize,
}

impl Scan {
    pub(super) fn found(
        &mut self,
        path: &DocumentPath,
        on_disk: Option<&Revision>,
        edit: Option<Revision>,
    ) {
        let was_live = on_disk.is_some_and(|revision| !revision.is_removal());

        self.live += usize::from(was_live && path.kind() != &DocumentKind::Project);
        self.edits.extend(edit);
    }

    pub(super) fn failed(&mut self, path: &DocumentPath, error: &SyncError) {
        self.problems.push(format!("{path}: {error}"));
    }

    pub(super) fn refuse_removing_everything(&self) -> Result<(), SyncError> {
        let removals = self
            .edits
            .iter()
            .filter(|edit| edit.is_removal() && !edit.parents.is_empty())
            .count();

        if removals < EVERYTHING_AT_LEAST || removals < self.live {
            return Ok(());
        }

        Err(SyncError::EverythingMissing(removals))
    }

    pub(super) fn record(
        self,
        mut record: impl FnMut(&Revision) -> Result<(), SyncError>,
    ) -> Recorded {
        let mut recorded = Recorded::new(self.problems);

        for edit in &self.edits {
            match record(edit) {
                Ok(()) => recorded.edits += 1,
                Err(error) => recorded.problems.push(format!("{}: {error}", edit.path)),
            }
        }

        recorded
    }
}
