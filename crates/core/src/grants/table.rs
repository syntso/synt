use rusqlite::Row;
use synt_journal::{ActionId, Actor, Clock, ResourceRef, SystemClock, Timestamp};

use super::{GrantId, GrantScope, StandingGrant};
use crate::CoreError;
use crate::log::log;
use crate::state::State;

const COLUMNS: &str = "id, actor, action, resource, destination, created_at";

#[derive(Debug, Clone)]
pub(crate) struct GrantTable {
    state: State,
}

impl GrantTable {
    pub(crate) fn new(state: State) -> Self {
        Self { state }
    }

    pub(crate) fn all(&self) -> Result<Vec<StandingGrant>, CoreError> {
        let connection = self.state.lock();
        let mut statement =
            connection.prepare(&format!("SELECT {COLUMNS} FROM grants ORDER BY id"))?;

        let rows = statement
            .query_map([], Self::row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows.into_iter().flatten().collect())
    }

    pub(crate) fn insert(&self, scope: GrantScope) -> Result<StandingGrant, CoreError> {
        let created_at = SystemClock.now();
        let actor = scope.actor().to_string();
        let connection = self.state.lock();

        connection.execute(
            "INSERT INTO grants (actor, action, resource, destination, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &actor,
                scope.action().as_str(),
                scope.target().map(ResourceRef::as_str),
                scope.destination(),
                created_at.as_millis(),
            ),
        )?;

        let id = Self::id(connection.last_insert_rowid());

        Ok(StandingGrant::new(id, scope, created_at))
    }

    pub(crate) fn delete(&self, id: GrantId) -> Result<(), CoreError> {
        let Ok(id) = i64::try_from(id.get()) else {
            return Ok(());
        };

        self.state
            .lock()
            .execute("DELETE FROM grants WHERE id = ?1", [id])?;

        Ok(())
    }

    pub(crate) fn clear(&self) -> Result<(), CoreError> {
        self.state.lock().execute("DELETE FROM grants", [])?;

        Ok(())
    }

    fn row(row: &Row<'_>) -> rusqlite::Result<Option<StandingGrant>> {
        let id = Self::id(row.get("id")?);
        let created_at = Timestamp::from_millis(row.get("created_at")?);

        let Some(scope) = Self::scope(row)? else {
            log!("grant {id} names an unknown actor or action; it is ignored");
            return Ok(None);
        };

        Ok(Some(StandingGrant::new(id, scope, created_at)))
    }

    fn scope(row: &Row<'_>) -> rusqlite::Result<Option<GrantScope>> {
        let actor = row.get::<_, String>("actor")?.parse::<Actor>().ok();
        let action = ActionId::new(row.get::<_, String>("action")?).ok();
        let resource: Option<String> = row.get("resource")?;
        let target = resource.map(ResourceRef::new).transpose().ok();
        let destination = row.get("destination")?;

        let (Some(actor), Some(action), Some(target)) = (actor, action, target) else {
            return Ok(None);
        };

        Ok(Some(GrantScope::new(actor, action, target, destination)))
    }

    fn id(row_id: i64) -> GrantId {
        GrantId::new(u64::try_from(row_id).unwrap_or_default())
    }
}
