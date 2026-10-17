use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_db::{Effect, TableName};
use synt_journal::ResourceRef;

use super::DbActions;
use crate::clock::milliseconds;

const LIMIT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub(crate) struct DbTables {
    databases: DbActions,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TablesResult {
    pub tables: Vec<TableName>,
    pub duration_ms: u64,
}

impl DbTables {
    pub(super) fn new(databases: DbActions) -> Self {
        Self { databases }
    }
}

impl Action for DbTables {
    const ID: &'static str = "db.tables";
    const TITLE: &'static str = "List the tables and views of a database";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ();
    type Output = TablesResult;

    fn check(&self, target: Option<&ResourceRef>, (): &()) -> Result<(), ActionError> {
        self.databases.check_target(target)?;

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, (): &()) -> Risk {
        self.databases.risk(target, Effect::Read)
    }

    fn destination(&self, target: Option<&ResourceRef>, (): &()) -> Option<String> {
        self.databases.destination(target)
    }

    fn summary(&self, output: &TablesResult) -> Value {
        json!({ "tables": output.tables.len(), "duration_ms": output.duration_ms })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<TablesResult, ActionError> {
        let started = Instant::now();

        let tables = self
            .databases
            .with(context, LIMIT, async |database| database.tables().await)
            .await?;

        Ok(TablesResult {
            tables,
            duration_ms: milliseconds(started.elapsed()),
        })
    }
}
