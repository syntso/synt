use crate::SyncError;
use crate::document::{DocumentKind, DocumentPath, Revision};

const EVERYTHING_AT_LEAST: usize = 2;

#[derive(Debug, Default)]
pub(crate) struct Scan {
    edits: Vec<Revision>,
    problems: Vec<String>,
    live: usize,
}

impl Scan {
    pub(crate) fn found(&mut self, path: &DocumentPath, was_live: bool, edit: Option<Revision>) {
        self.live += usize::from(was_live && path.kind() != &DocumentKind::Project);
        self.edits.extend(edit);
    }

    pub(crate) fn failed(&mut self, path: &DocumentPath, error: &SyncError) {
        self.problems.push(format!("{path}: {error}"));
    }

    pub(crate) fn refuse_removing_everything(&self) -> Result<(), SyncError> {
        let removals = self
            .edits
            .iter()
            .filter(|edit| edit.is_removal() && !edit.parents.is_empty())
            .count();

        if removals < EVERYTHING_AT_LEAST || removals < self.live {
            return Ok(());
        }

        Err(SyncError::Project(format!(
            "every one of the {removals} documents of the project is missing from its folder; \
             the round is skipped so the other members keep them. Put the files back, remove them \
             a few at a time, or run sync.disable to stop syncing this project"
        )))
    }

    pub(crate) fn into_parts(self) -> (Vec<Revision>, Vec<String>) {
        (self.edits, self.problems)
    }
}
