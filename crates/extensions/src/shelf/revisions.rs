mod stored;

use rusqlite::{OptionalExtension, Row};

pub(crate) use stored::StoredRevision;

use super::Sqlite;
use crate::ExtensionError;
use crate::sha256::Sha256;

const KEPT_REVISIONS: i64 = 32;

#[derive(Debug, Clone)]
pub(crate) struct Revisions {
    sqlite: Sqlite,
}

impl Revisions {
    pub(super) fn new(sqlite: Sqlite) -> Self {
        Self { sqlite }
    }

    pub(crate) fn find(
        &self,
        project: &str,
        path: &str,
        revision: &str,
    ) -> Result<Option<StoredRevision>, ExtensionError> {
        let found = self
            .sqlite
            .lock()
            .query_row(
                "SELECT revision, parents, body FROM extension_revisions \
                 WHERE project = ?1 AND path = ?2 AND revision = ?3",
                [project, path, revision],
                stored,
            )
            .optional()?;

        Ok(found)
    }

    pub(crate) fn head(&self, project: &str, path: &str) -> Result<Option<Sha256>, ExtensionError> {
        let found = self
            .sqlite
            .lock()
            .query_row(
                "SELECT revision FROM extension_revisions WHERE project = ?1 AND path = ?2 \
                 ORDER BY seq DESC LIMIT 1",
                [project, path],
                |row| row.get("revision"),
            )
            .optional()?;

        Ok(found)
    }

    pub(crate) fn add(
        &self,
        project: &str,
        path: &str,
        revision: &StoredRevision,
    ) -> Result<(), ExtensionError> {
        let mut connection = self.sqlite.lock();
        let transaction = connection.transaction()?;

        let last: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(seq), 0) FROM extension_revisions WHERE project = ?1 AND path = ?2",
            [project, path],
            |row| row.get(0),
        )?;

        transaction.execute(
            "INSERT OR REPLACE INTO extension_revisions (project, path, revision, parents, body, seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                project,
                path,
                revision.revision(),
                revision.joined_parents(),
                revision.body(),
                last + 1,
            ),
        )?;
        transaction.execute(
            "DELETE FROM extension_revisions WHERE project = ?1 AND path = ?2 AND seq <= ?3",
            (project, path, last + 1 - KEPT_REVISIONS),
        )?;
        transaction.commit()?;

        Ok(())
    }

    pub(crate) fn forget(&self, project: &str, path: &str) -> Result<(), ExtensionError> {
        self.sqlite.lock().execute(
            "DELETE FROM extension_revisions WHERE project = ?1 AND path = ?2",
            [project, path],
        )?;

        Ok(())
    }
}

fn stored(row: &Row<'_>) -> rusqlite::Result<StoredRevision> {
    let parents: String = row.get("parents")?;

    Ok(StoredRevision::recorded(
        row.get("revision")?,
        &parents,
        row.get("body")?,
    ))
}

#[cfg(test)]
mod tests {
    use crate::sha256::Sha256;
    use crate::shelf::{Shelf, StoredRevision};

    fn revision(name: &str, parents: &[&str]) -> StoredRevision {
        StoredRevision::recorded(
            Sha256::recorded(name),
            &parents.join(","),
            name.as_bytes().to_vec(),
        )
    }

    #[test]
    fn the_newest_revision_is_the_head_and_old_ones_are_trimmed() {
        let (_folder, shelf) = Shelf::scratch();
        let revisions = shelf.revisions();

        revisions.add("p", "a.uml", &revision("r0", &[])).unwrap();
        for index in 1..40 {
            let parent = format!("r{}", index - 1);
            let name = format!("r{index}");
            revisions
                .add("p", "a.uml", &revision(&name, &[&parent]))
                .unwrap();
        }

        let newest = revisions.find("p", "a.uml", "r39").unwrap().unwrap();

        assert_eq!(
            revisions.head("p", "a.uml").unwrap(),
            Some(Sha256::recorded("r39"))
        );
        assert_eq!(newest.parents(), [Sha256::recorded("r38")]);
        assert!(revisions.find("p", "a.uml", "r0").unwrap().is_none());
        assert!(revisions.find("p", "a.uml", "r8").unwrap().is_some());
        assert_eq!(revisions.head("p", "b.uml").unwrap(), None);
    }
}
