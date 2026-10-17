use rusqlite::{Connection, OpenFlags};

use super::Access;
use crate::error::JournalError;

pub(crate) struct ReadOnly;

impl Access for ReadOnly {
    const FLAGS: OpenFlags =
        OpenFlags::SQLITE_OPEN_READ_ONLY.union(OpenFlags::SQLITE_OPEN_NO_MUTEX);

    fn prepare(_connection: &Connection) -> Result<(), JournalError> {
        Ok(())
    }
}
