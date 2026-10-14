use serde::Serialize;

use super::RunbookSummary;
use crate::projects::Problem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunbookListing {
    pub project: String,
    pub runbooks: Vec<RunbookSummary>,
    pub problems: Vec<Problem>,
}
