use serde::Serialize;

use crate::runbooks::document::Runbook;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunbookEntry {
    pub name: String,
    pub runbook: Runbook,
}
