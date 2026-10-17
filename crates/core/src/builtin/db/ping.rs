use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_db::{Effect, Engine};
use synt_journal::ResourceRef;

use super::Databases;
use crate::clock::milliseconds;

const LIMIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub(crate) struct DbPing {
    databases: Databases,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PingResult {
    pub engine: Engine,
    pub version: String,
    pub duration_ms: u64,
}

impl DbPing {
    pub(super) fn new(databases: Databases) -> Self {
        Self { databases }
    }
}

impl Action for DbPing {
    const ID: &'static str = "db.ping";
    const TITLE: &'static str = "Check that a database answers, and which version it runs";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ();
    type Output = PingResult;

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

    fn summary(&self, output: &PingResult) -> Value {
        json!({
            "engine": output.engine,
            "version": output.version,
            "duration_ms": output.duration_ms,
        })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<PingResult, ActionError> {
        let engine = self.databases.current_engine(context)?;
        let started = Instant::now();

        let version = self
            .databases
            .with(context, LIMIT, async |database| database.ping().await)
            .await?;

        Ok(PingResult {
            engine,
            version,
            duration_ms: milliseconds(started.elapsed()),
        })
    }
}
