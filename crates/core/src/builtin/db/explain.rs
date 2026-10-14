use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_db::{Classification, Effect, Engine};
use synt_journal::ResourceRef;

use super::Databases;
use super::recorded::recorded_statement;
use crate::clock::milliseconds;

const LIMIT: Duration = Duration::from_secs(60);
const EXPLAIN: &str = "explain";

#[derive(Debug, Clone)]
pub(crate) struct DbExplain {
    databases: Databases,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExplainParams {
    pub sql: String,

    #[serde(default)]
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ExplainResult {
    pub kind: String,
    pub plan: Value,
    pub duration_ms: u64,
}

impl DbExplain {
    pub(super) fn new(databases: Databases) -> Self {
        Self { databases }
    }
}

impl Action for DbExplain {
    const ID: &'static str = "db.explain";
    const TITLE: &'static str = "Show how a database would run a statement, without running it";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ExplainParams;
    type Output = ExplainResult;

    fn check(
        &self,
        target: Option<&ResourceRef>,
        params: &ExplainParams,
    ) -> Result<(), ActionError> {
        let engine = self.databases.check_target(target)?;

        one_statement(engine, &params.sql).map(|_| ())
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &ExplainParams) -> Risk {
        self.databases.risk(target, Effect::Read)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ExplainParams) -> Option<String> {
        self.databases.destination(target)
    }

    fn recorded_params(&self, params: &Value) -> Value {
        recorded_statement(params)
    }

    fn summary(&self, output: &ExplainResult) -> Value {
        json!({ "kind": output.kind, "duration_ms": output.duration_ms })
    }

    async fn run(
        &self,
        context: &Context,
        params: ExplainParams,
    ) -> Result<ExplainResult, ActionError> {
        let engine = self
            .databases
            .engine(context.target())
            .ok_or_else(|| ActionError::failed("the target is no longer a database"))?;
        let classified = one_statement(engine, &params.sql)?;
        let started = Instant::now();

        let plan = self
            .databases
            .with(context, LIMIT, async |database| {
                database.explain(&params.sql, &params.params).await
            })
            .await?;

        Ok(ExplainResult {
            kind: classified.kind,
            plan,
            duration_ms: milliseconds(started.elapsed()),
        })
    }
}

fn one_statement(engine: Engine, sql: &str) -> Result<Classification, ActionError> {
    if !engine.speaks_sql() {
        return Err(ActionError::InvalidParams(
            "redis has no query plans to explain".to_owned(),
        ));
    }

    let classified = Classification::of_sql(engine, sql);

    if !classified.is_one_statement() {
        return Err(ActionError::InvalidParams(format!(
            "db.explain takes one statement the parser understands, got {}",
            classified.kind
        )));
    }

    if classified.kind == EXPLAIN {
        return Err(ActionError::InvalidParams(
            "give the statement itself; db.explain adds EXPLAIN".to_owned(),
        ));
    }

    Ok(classified)
}
