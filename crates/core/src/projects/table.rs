use std::path::{Path, PathBuf};

use synt_journal::{Clock, SystemClock};

use crate::CoreError;
use crate::state::State;

#[derive(Debug, Clone)]
pub(crate) struct ProjectTable {
    state: State,
}

impl ProjectTable {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }

    pub(crate) fn open(&self) -> Result<Vec<PathBuf>, CoreError> {
        let connection = self.state.lock();
        let mut statement =
            connection.prepare("SELECT path FROM open_projects ORDER BY opened_at, path")?;

        let paths = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(paths.into_iter().map(PathBuf::from).collect())
    }

    pub(crate) fn remember(&self, path: &Path) -> Result<(), CoreError> {
        self.state.lock().execute(
            "INSERT OR IGNORE INTO open_projects (path, opened_at) VALUES (?1, ?2)",
            (path.to_string_lossy(), SystemClock.now().as_millis()),
        )?;

        Ok(())
    }

    pub(crate) fn forget(&self, path: &Path) -> Result<(), CoreError> {
        self.state.lock().execute(
            "DELETE FROM open_projects WHERE path = ?1",
            [path.to_string_lossy()],
        )?;

        Ok(())
    }
}
