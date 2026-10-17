mod read_only;
mod writable;

use rusqlite::{Connection, OpenFlags};

use crate::error::JournalError;

pub(crate) use read_only::ReadOnly;
pub(crate) use writable::Writable;

pub(crate) trait Access {
    const FLAGS: OpenFlags;

    fn prepare(connection: &Connection) -> Result<(), JournalError>;
}
