use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{StoredDocument, SyncedProject, schema};
use crate::SyncError;
use crate::clock::unix_millis;
use crate::document::{DocumentPath, Revision};
use crate::id::{DocumentId, MemberId, ProjectId, RevisionId};
use crate::merge::DocumentGraph;
use crate::provider::{Cursor, Place, SealedRevision};

const BUSY_WAIT: Duration = Duration::from_secs(5);
const REVISION_COLUMNS: &str = "revision, document, parents, at, path, body, conflicts, author";

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
                synced_project,
            )
            .optional()?;

        Ok(synced)
    }

    pub(crate) fn synced_projects(&self) -> Result<Vec<SyncedProject>, SyncError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare("SELECT project, place, cursor FROM sync_projects ORDER BY enabled_at")?;

        let projects = statement
            .query_map([], synced_project)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    pub(crate) fn enable(&self, project: &ProjectId, place: &Place) -> Result<(), SyncError> {
        self.lock().execute(
            "INSERT OR IGNORE INTO sync_projects (project, place, cursor, enabled_at) \
             VALUES (?1, ?2, '', ?3)",
            params![project.get(), json(place)?, signed(unix_millis())],
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
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        for table in [
            "sync_projects",
            "sync_revisions",
            "sync_documents",
            "sync_held",
        ] {
            transaction.execute(
                &format!("DELETE FROM {table} WHERE project = ?1"),
                [project.get()],
            )?;
        }

        transaction.commit()?;

        Ok(())
    }

    pub(crate) fn insert(
        &self,
        project: &ProjectId,
        revision: &Revision,
        pushed: bool,
    ) -> Result<bool, SyncError> {
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        let inserted = transaction.execute(
            &format!(
                "INSERT OR IGNORE INTO sync_revisions (project, {REVISION_COLUMNS}, pushed) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            ),
            params![
                project.get(),
                revision.id.get(),
                revision.document.get(),
                json(&revision.parents)?,
                signed(revision.at),
                revision.path.get(),
                revision.body,
                json(&revision.conflicts)?,
                revision.author.as_ref().map(MemberId::get),
                pushed,
            ],
        )?;

        transaction.execute(
            "INSERT OR IGNORE INTO sync_documents (project, document, path) VALUES (?1, ?2, ?3)",
            [project.get(), revision.document.get(), revision.path.get()],
        )?;

        transaction.commit()?;

        Ok(inserted > 0)
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
        let revisions = self.revisions(
            &format!(
                "SELECT {REVISION_COLUMNS} FROM sync_revisions \
                 WHERE project = ?1 AND document = ?2"
            ),
            [project.get(), document.get()],
        )?;

        Ok(DocumentGraph::new(revisions))
    }

    pub(crate) fn unpushed(&self, project: &ProjectId) -> Result<Vec<Revision>, SyncError> {
        self.revisions(
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
        let mut connection = self.lock();
        let transaction = connection.transaction()?;

        for revision in revisions {
            transaction.execute(
                "UPDATE sync_revisions SET pushed = 1 WHERE project = ?1 AND revision = ?2",
                [project.get(), revision.get()],
            )?;
        }

        transaction.commit()?;

        Ok(())
    }

    pub(crate) fn documents(&self, project: &ProjectId) -> Result<Vec<StoredDocument>, SyncError> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT document, path, on_disk FROM sync_documents WHERE project = ?1 ORDER BY path",
        )?;

        let documents = statement
            .query_map([project.get()], stored_document)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(documents)
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
            [project.get(), sealed.revision.get(), &json(sealed)?],
        )?;

        Ok(())
    }

    pub(crate) fn held(&self, project: &ProjectId) -> Result<Vec<SealedRevision>, SyncError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare("SELECT sealed FROM sync_held WHERE project = ?1 ORDER BY revision")?;

        let held = statement
            .query_map([project.get()], |row| parsed(row, 0, from_json))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(held)
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

    fn revisions(
        &self,
        query: &str,
        parameters: impl rusqlite::Params,
    ) -> Result<Vec<Revision>, SyncError> {
        let connection = self.lock();
        let mut statement = connection.prepare(query)?;

        let revisions = statement
            .query_map(parameters, revision)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(revisions)
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn synced_project(row: &Row<'_>) -> rusqlite::Result<SyncedProject> {
    Ok(SyncedProject {
        project: parsed(row, 0, ProjectId::parse)?,
        place: parsed(row, 1, from_json)?,
        cursor: Cursor::new(row.get::<_, String>(2)?),
    })
}

fn stored_document(row: &Row<'_>) -> rusqlite::Result<StoredDocument> {
    let on_disk: Option<String> = row.get(2)?;

    Ok(StoredDocument {
        document: parsed(row, 0, DocumentId::parse)?,
        path: parsed(row, 1, DocumentPath::parse)?,
        on_disk: on_disk
            .map(|text| RevisionId::parse(&text).map_err(|error| damaged(2, error)))
            .transpose()?,
    })
}

fn revision(row: &Row<'_>) -> rusqlite::Result<Revision> {
    let author: Option<String> = row.get(7)?;

    Ok(Revision {
        id: parsed(row, 0, RevisionId::parse)?,
        document: parsed(row, 1, DocumentId::parse)?,
        parents: parsed(row, 2, from_json)?,
        at: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
        path: parsed(row, 4, DocumentPath::parse)?,
        body: row.get(5)?,
        conflicts: parsed(row, 6, from_json)?,
        author: author
            .map(|text| MemberId::parse(&text).map_err(|error| damaged(7, error)))
            .transpose()?,
    })
}

fn parsed<T>(
    row: &Row<'_>,
    column: usize,
    parse: impl Fn(&str) -> Result<T, SyncError>,
) -> rusqlite::Result<T> {
    let text: String = row.get(column)?;

    parse(&text).map_err(|error| damaged(column, error))
}

fn damaged(column: usize, error: SyncError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}

fn json(value: &impl serde::Serialize) -> Result<String, SyncError> {
    serde_json::to_string(value).map_err(|error| SyncError::invalid(error.to_string()))
}

fn from_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, SyncError> {
    serde_json::from_str(text).map_err(|error| SyncError::invalid(error.to_string()))
}

fn signed(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
