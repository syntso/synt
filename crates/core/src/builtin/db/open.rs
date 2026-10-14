use synt_db::Database;
use tokio_util::sync::DropGuard;

pub(super) struct OpenDatabase {
    pub(super) database: Database,
    _tunnel: Option<DropGuard>,
}

impl OpenDatabase {
    pub(super) fn new(database: Database, tunnel: Option<DropGuard>) -> Self {
        Self {
            database,
            _tunnel: tunnel,
        }
    }

    pub(super) async fn close(self) {
        let Self { database, _tunnel } = self;

        database.close().await;
    }
}
