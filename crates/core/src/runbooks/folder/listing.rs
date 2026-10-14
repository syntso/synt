use serde::Serialize;

use super::RunbookSummary;
use crate::projects::Problem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunbookListing {
    project: String,
    runbooks: Vec<RunbookSummary>,
    problems: Vec<Problem>,
}

impl RunbookListing {
    pub(crate) fn new(project: &str) -> Self {
        Self {
            project: project.to_owned(),
            runbooks: Vec::new(),
            problems: Vec::new(),
        }
    }

    pub(crate) fn project(&self) -> &str {
        &self.project
    }

    pub(crate) fn runbooks(&self) -> &[RunbookSummary] {
        &self.runbooks
    }

    pub(crate) fn problems(&self) -> &[Problem] {
        &self.problems
    }

    pub(crate) fn found(&mut self, summary: RunbookSummary) {
        self.runbooks.push(summary);
    }

    pub(crate) fn unreadable(&mut self, problem: Problem) {
        self.problems.push(problem);
    }
}
