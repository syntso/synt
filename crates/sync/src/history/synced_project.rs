use crate::id::ProjectId;
use crate::provider::{Cursor, Place};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncedProject {
    pub(crate) project: ProjectId,
    pub(crate) place: Place,
    pub(crate) cursor: Cursor,
}
