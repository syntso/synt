use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{Connection, InterruptHandle, OpenFlags};

use crate::{Access, DbError};

const BUSY_LIMIT: Duration = Duration::from_secs(5);

pub(super) struct Opened {
    access: Access,
    connection: Arc<Mutex<Connection>>,
    interrupt: InterruptHandle,
}

impl Opened {
    pub(super) fn open(path: &Path, access: Access) -> Result<Self, DbError> {
        let connection =
            Connection::open_with_flags(path, Self::flags(access)).map_err(|source| {
                DbError::Unopened {
                    path: path.to_owned(),
                    source,
                }
            })?;

        connection.busy_timeout(BUSY_LIMIT)?;

        if access.is_read_only() {
            connection.pragma_update(None, "query_only", true)?;
        }

        Ok(Self {
            access,
            interrupt: connection.get_interrupt_handle(),
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(super) fn serves(&self, access: Access) -> bool {
        self.access == access
    }

    pub(super) fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.connection)
    }

    pub(super) fn interrupt(&self) {
        self.interrupt.interrupt();
    }

    fn flags(access: Access) -> OpenFlags {
        let mode = match access {
            Access::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
            Access::ReadWrite => OpenFlags::SQLITE_OPEN_READ_WRITE,
        };

        mode | OpenFlags::SQLITE_OPEN_NO_MUTEX
    }
}
