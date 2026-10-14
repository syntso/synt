use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::id::ProjectId;
use crate::report::RoundReport;

#[derive(Debug, Default)]
pub(crate) struct Reports {
    last: Mutex<BTreeMap<ProjectId, RoundReport>>,
}

impl Reports {
    pub(crate) fn record(&self, project: &ProjectId, report: &RoundReport) {
        self.lock().insert(project.clone(), report.clone());
    }

    pub(crate) fn forget(&self, project: &ProjectId) {
        self.lock().remove(project);
    }

    pub(crate) fn last(&self, project: &ProjectId) -> Option<RoundReport> {
        self.lock().get(project).cloned()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<ProjectId, RoundReport>> {
        self.last.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
