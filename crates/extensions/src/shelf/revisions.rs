use rusqlite::{OptionalExtension, Row};

use super::{Shelf, StoredRevision};
use crate::ExtensionError;
use crate::sha256::Sha256;

const KEPT_REVISIONS: i64 = 32;

impl Shelf {
    pub(crate) fn revision(
        &self,
        project: &str,
        path: &str,
        revision: &str,
    ) -> Result<Option<StoredRevision>, ExtensionError> {
        let found = self
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

    pub(crate) fn add_revision(
        &self,
        project: &str,
        path: &str,
        revision: &StoredRevision,
    ) -> Result<(), ExtensionError> {
        let mut connection = self.lock();
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
                &revision.revision,
                joined(&revision.parents),
                &revision.body,
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

    pub(crate) fn forget_revisions(&self, project: &str, path: &str) -> Result<(), ExtensionError> {
        self.lock().execute(
            "DELETE FROM extension_revisions WHERE project = ?1 AND path = ?2",
            [project, path],
        )?;

        Ok(())
    }
}

fn stored(row: &Row<'_>) -> rusqlite::Result<StoredRevision> {
    let parents: String = row.get("parents")?;

    Ok(StoredRevision {
        revision: row.get("revision")?,
        parents: parents
            .split(',')
            .filter(|parent| !parent.is_empty())
            .map(Sha256::recorded)
            .collect(),
        body: row.get("body")?,
    })
}

fn joined(parents: &[Sha256]) -> String {
    let parents: Vec<&str> = parents.iter().map(Sha256::as_str).collect();

    parents.join(",")
}

#[cfg(test)]
mod tests {
    use crate::sha256::Sha256;
    use crate::shelf::{Shelf, StoredRevision};

    fn revision(name: &str, parents: &[&str]) -> StoredRevision {
        StoredRevision {
            revision: Sha256::recorded(name),
            parents: parents.iter().copied().map(Sha256::recorded).collect(),
            body: name.as_bytes().to_vec(),
        }
    }

    #[test]
    fn the_newest_revision_is_the_head_and_old_ones_are_trimmed() {
        let folder = tempfile::tempdir().unwrap();
        let shelf = Shelf::open(&folder.path().join("state.sqlite")).unwrap();

        shelf
            .add_revision("p", "a.uml", &revision("r0", &[]))
            .unwrap();
        for index in 1..40 {
            let parent = format!("r{}", index - 1);
            let name = format!("r{index}");
            shelf
                .add_revision("p", "a.uml", &revision(&name, &[&parent]))
                .unwrap();
        }

        assert_eq!(
            shelf.head("p", "a.uml").unwrap(),
            Some(Sha256::recorded("r39"))
        );
        assert_eq!(
            shelf
                .revision("p", "a.uml", "r39")
                .unwrap()
                .unwrap()
                .parents,
            [Sha256::recorded("r38")]
        );
        assert!(shelf.revision("p", "a.uml", "r0").unwrap().is_none());
        assert!(shelf.revision("p", "a.uml", "r8").unwrap().is_some());
        assert_eq!(shelf.head("p", "b.uml").unwrap(), None);
    }
}
