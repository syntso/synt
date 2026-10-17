mod origin;
mod row;
mod schema;
mod stored_document;
mod synced_project;

pub(crate) use origin::Origin;
pub(crate) use stored_document::StoredDocument;
pub(crate) use synced_project::SyncedProject;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Params, Row, Transaction, params};

use crate::SyncError;
use crate::document::Revision;
use crate::id::{DocumentId, MemberId, ProjectId, RevisionId};
use crate::merge::DocumentGraph;
use crate::provider::{Cursor, Place, SealedRevision};
use crate::report::ConflictInfo;
use row::{FromRow, TypedRow};
use synt_journal::Timestamp;

const BUSY_WAIT: Duration = Duration::from_secs(5);
const REVISION_COLUMNS: &str = "revision, document, parents, at, path, body, conflicts, author";
const PROJECT_TABLES: [&str; 4] = [
    "sync_projects",
    "sync_revisions",
    "sync_documents",
    "sync_held",
];

#[derive(Debug, Clone)]
pub(crate) struct History {
    connection: Arc<Mutex<Connection>>,
}

impl History {
    pub(crate) fn open(path: &Path) -> Result<Self, SyncError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(BUSY_WAIT)?;
        connection.execute_batch(schema::CREATE)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(crate) fn synced(&self, project: &ProjectId) -> Result<Option<SyncedProject>, SyncError> {
        let synced = self
            .lock()
            .query_row(
                "SELECT project, place, cursor FROM sync_projects WHERE project = ?1",
                [project.get()],
                SyncedProject::from_row,
            )
            .optional()?;

        Ok(synced)
    }

    pub(crate) fn synced_projects(&self) -> Result<Vec<SyncedProject>, SyncError> {
        self.all(
            "SELECT project, place, cursor FROM sync_projects ORDER BY enabled_at",
            [],
        )
    }

    pub(crate) fn enable(
        &self,
        project: &ProjectId,
        place: &Place,
        at: Timestamp,
    ) -> Result<(), SyncError> {
        self.lock().execute(
            "INSERT OR IGNORE INTO sync_projects (project, place, cursor, enabled_at) \
             VALUES (?1, ?2, '', ?3)",
            params![project.get(), serde_json::to_string(place)?, at.as_millis()],
        )?;

        Ok(())
    }

    pub(crate) fn set_cursor(&self, project: &ProjectId, cursor: &Cursor) -> Result<(), SyncError> {
        self.lock().execute(
            "UPDATE sync_projects SET cursor = ?2 WHERE project = ?1",
            [project.get(), cursor.get()],
        )?;

        Ok(())
    }

    pub(crate) fn forget(&self, project: &ProjectId) -> Result<(), SyncError> {
        self.in_transaction(|transaction| {
            for table in PROJECT_TABLES {
                transaction.execute(
                    &format!("DELETE FROM {table} WHERE project = ?1"),
                    [project.get()],
                )?;
            }

            Ok(())
        })
    }

    pub(crate) fn insert(
        &self,
        project: &ProjectId,
        revision: &Revision,
        origin: Origin,
    ) -> Result<bool, SyncError> {
        self.in_transaction(|transaction| {
            let inserted = transaction.execute(
                &format!(
                    "INSERT OR IGNORE INTO sync_revisions (project, {REVISION_COLUMNS}, pushed) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
                ),
                params![
                    project.get(),
                    revision.id.get(),
                    revision.document.get(),
                    serde_json::to_string(&revision.parents)?,
                    signed(revision.at),
                    revision.path.get(),
                    revision.body,
                    serde_json::to_string(&revision.conflicts)?,
                    revision.author.as_ref().map(MemberId::get),
                    origin.is_pushed(),
                ],
            )?;

            transaction.execute(
                "INSERT OR IGNORE INTO sync_documents (project, document, path) VALUES (?1, ?2, ?3)",
                [project.get(), revision.document.get(), revision.path.get()],
            )?;

            Ok(inserted > 0)
        })
    }

    pub(crate) fn contains(
        &self,
        project: &ProjectId,
        revision: &RevisionId,
    ) -> Result<bool, SyncError> {
        let found: Option<i64> = self
            .lock()
            .query_row(
                "SELECT 1 FROM sync_revisions WHERE project = ?1 AND revision = ?2",
                [project.get(), revision.get()],
                |row| row.get(0),
            )
            .optional()?;

        Ok(found.is_some())
    }

    pub(crate) fn graph(
        &self,
        project: &ProjectId,
        document: &DocumentId,
    ) -> Result<DocumentGraph, SyncError> {
        let revisions = self.all(
            &format!(
                "SELECT {REVISION_COLUMNS} FROM sync_revisions \
                 WHERE project = ?1 AND document = ?2"
            ),
            [project.get(), document.get()],
        )?;

        Ok(DocumentGraph::new(revisions))
    }

    pub(crate) fn conflicts(&self, project: &ProjectId) -> Result<Vec<ConflictInfo>, SyncError> {
        let mut conflicts = Vec::new();

        for stored in self.documents(project)? {
            let graph = self.graph(project, &stored.document)?;

            conflicts.extend(graph.conflict(&stored.path));
        }

        Ok(conflicts)
    }

    pub(crate) fn unpushed(&self, project: &ProjectId) -> Result<Vec<Revision>, SyncError> {
        self.all(
            &format!(
                "SELECT {REVISION_COLUMNS} FROM sync_revisions \
                 WHERE project = ?1 AND pushed = 0 ORDER BY at, revision"
            ),
            [project.get()],
        )
    }

    pub(crate) fn mark_pushed(
        &self,
        project: &ProjectId,
        revisions: &[RevisionId],
    ) -> Result<(), SyncError> {
        self.in_transaction(|transaction| {
            for revision in revisions {
                transaction.execute(
                    "UPDATE sync_revisions SET pushed = 1 WHERE project = ?1 AND revision = ?2",
                    [project.get(), revision.get()],
                )?;
            }

            Ok(())
        })
    }

    pub(crate) fn documents(&self, project: &ProjectId) -> Result<Vec<StoredDocument>, SyncError> {
        self.all(
            "SELECT document, path, on_disk FROM sync_documents WHERE project = ?1 ORDER BY path",
            [project.get()],
        )
    }

    pub(crate) fn set_on_disk(
        &self,
        project: &ProjectId,
        revision: &Revision,
    ) -> Result<(), SyncError> {
        self.lock().execute(
            "INSERT INTO sync_documents (project, document, path, on_disk) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (project, document) DO UPDATE SET on_disk = excluded.on_disk",
            [
                project.get(),
                revision.document.get(),
                revision.path.get(),
                revision.id.get(),
            ],
        )?;

        Ok(())
    }

    pub(crate) fn hold(
        &self,
        project: &ProjectId,
        sealed: &SealedRevision,
    ) -> Result<(), SyncError> {
        self.lock().execute(
            "INSERT OR IGNORE INTO sync_held (project, revision, sealed) VALUES (?1, ?2, ?3)",
            [
                project.get(),
                sealed.revision.get(),
                &serde_json::to_string(sealed)?,
            ],
        )?;

        Ok(())
    }

    pub(crate) fn held(&self, project: &ProjectId) -> Result<Vec<SealedRevision>, SyncError> {
        self.all(
            "SELECT sealed FROM sync_held WHERE project = ?1 ORDER BY revision",
            [project.get()],
        )
    }

    pub(crate) fn release(
        &self,
        project: &ProjectId,
        revision: &RevisionId,
    ) -> Result<(), SyncError> {
        self.lock().execute(
            "DELETE FROM sync_held WHERE project = ?1 AND revision = ?2",
            [project.get(), revision.get()],
        )?;

        Ok(())
    }

    fn all<T: FromRow>(&self, query: &str, parameters: impl Params) -> Result<Vec<T>, SyncError> {
        let connection = self.lock();
        let mut statement = connection.prepare(query)?;

        let rows = statement
            .query_map(parameters, T::from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    fn in_transaction<T>(
        &self,
        work: impl FnOnce(&Transaction<'_>) -> Result<T, SyncError>,
    ) -> Result<T, SyncError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        let done = work(&transaction)?;
        transaction.commit()?;

        Ok(done)
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl FromRow for Revision {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let at: i64 = row.get("at")?;

        Ok(Self {
            id: row.parsed("revision")?,
            document: row.parsed("document")?,
            parents: row.json("parents")?,
            at: u64::try_from(at).unwrap_or_default(),
            path: row.parsed("path")?,
            body: row.get("body")?,
            conflicts: row.json("conflicts")?,
            author: row.parsed_or_null("author")?,
        })
    }
}

impl FromRow for SealedRevision {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        row.json("sealed")
    }
}

fn signed(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
