use crate::SyncError;
use crate::document::Revision;
use crate::history::{History, SyncedProject};
use crate::id::RevisionId;
use crate::provider::{SealedRevision, SyncProvider};
use crate::replica::Replica;
use crate::report::RoundReport;
use crate::seal::Keyring;

pub(super) struct Round<'a> {
    pub(super) history: &'a History,
    pub(super) synced: &'a SyncedProject,
    pub(super) provider: &'a dyn SyncProvider,
    pub(super) keyring: &'a Keyring,
    pub(super) replica: &'a Replica,
}

impl Round<'_> {
    pub(super) fn run(&self, report: &mut RoundReport) -> Result<(), SyncError> {
        let recorded = self.replica.record_edits()?;
        report.recorded = recorded.edits;
        report.problems = recorded.problems;

        self.pull(report)?;

        let settled = self.replica.settle()?;
        report.applied = settled.applied;
        report.merged = settled.merged;
        report.problems.extend(settled.problems);

        report.pushed = self.push()?;

        Ok(())
    }

    fn pull(&self, report: &mut RoundReport) -> Result<(), SyncError> {
        let project = &self.synced.project;
        let (fresh, cursor) = self.provider.pull(project, &self.synced.cursor)?;
        let held = self.history.held(project)?;

        for sealed in held.into_iter().chain(fresh) {
            self.receive(&sealed, report)?;
        }

        self.history.set_cursor(project, &cursor)
    }

    fn receive(&self, sealed: &SealedRevision, report: &mut RoundReport) -> Result<(), SyncError> {
        let project = &self.synced.project;

        if self.history.contains(project, &sealed.revision)? {
            return self.history.release(project, &sealed.revision);
        }

        if !self.keyring.holds(sealed.generation) {
            report.held += 1;
            return self.history.hold(project, sealed);
        }

        self.history.release(project, &sealed.revision)?;

        match Revision::open(project, self.keyring, sealed) {
            Ok(revision) => report.pulled += usize::from(self.replica.accept(&revision)?),
            Err(error) => {
                report.refused += 1;
                report
                    .problems
                    .push(format!("revision {}: {error}", sealed.revision));
            }
        }

        Ok(())
    }

    fn push(&self) -> Result<usize, SyncError> {
        let project = &self.synced.project;
        let unpushed = self.history.unpushed(project)?;

        if unpushed.is_empty() {
            return Ok(0);
        }

        let sealed = unpushed
            .iter()
            .map(|revision| revision.seal(project, self.keyring))
            .collect::<Result<Vec<_>, _>>()?;
        self.provider.push(project, &sealed)?;

        let pushed: Vec<RevisionId> = unpushed.into_iter().map(|revision| revision.id).collect();
        self.history.mark_pushed(project, &pushed)?;

        Ok(pushed.len())
    }
}
