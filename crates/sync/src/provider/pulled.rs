use super::{Cursor, SealedRevision};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pulled {
    pub revisions: Vec<SealedRevision>,
    pub cursor: Cursor,
}
