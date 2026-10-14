use synt_db::{Database, DbError};
use tokio_util::sync::DropGuard;

pub(super) struct OpenDatabase {
    database: Database,
    _tunnel: Option<DropGuard>,
}

impl OpenDatabase {
    pub(super) fn new(database: Database, tunnel: Option<DropGuard>) -> Self {
        Self {
            database,
            _tunnel: tunnel,
        }
    }

    pub(super) async fn serve<T>(
        mut self,
        work: impl AsyncFnOnce(&mut Database) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        let done = work(&mut self.database).await;

        let Self { database, _tunnel } = self;
        database.close().await;

        done
    }
}
