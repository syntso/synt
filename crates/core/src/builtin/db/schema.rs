use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_db::{Effect, Table};
use synt_journal::ResourceRef;

use super::Databases;
use crate::clock::milliseconds;

const LIMIT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub(crate) struct DbSchema {
    databases: Databases,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SchemaResult {
    pub tables: Vec<Table>,
    pub duration_ms: u64,
}

impl DbSchema {
    pub(super) fn new(databases: Databases) -> Self {
        Self { databases }
    }
}

impl Action for DbSchema {
    const ID: &'static str = "db.schema";
    const TITLE: &'static str =
        "Describe a database: tables, columns, types, keys, indexes and foreign keys";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ();
    type Output = SchemaResult;

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

    fn summary(&self, output: &SchemaResult) -> Value {
        let columns: usize = output.tables.iter().map(|table| table.columns.len()).sum();

        json!({
            "tables": output.tables.len(),
            "columns": columns,
            "duration_ms": output.duration_ms,
        })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<SchemaResult, ActionError> {
        let started = Instant::now();

        let schema = self
            .databases
            .with(context, LIMIT, async |database| database.schema().await)
            .await?;

        Ok(SchemaResult {
            tables: schema.tables,
            duration_ms: milliseconds(started.elapsed()),
        })
    }
}
