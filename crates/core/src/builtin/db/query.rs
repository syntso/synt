use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_db::{Access, Classification, Effect, Engine, Page, Query, Rows};
use synt_journal::ResourceRef;

use super::Databases;
use super::recorded::recorded_statement;

const DEFAULT_ROWS: usize = 100;
const MOST_ROWS: usize = 10_000;
const DEFAULT_BYTES: usize = 1024 * 1024;
const MOST_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_TIMEOUT: u64 = 60;
const LONGEST_TIMEOUT: u64 = 3600;

#[derive(Debug, Clone)]
pub(crate) struct DbQuery {
    databases: Databases,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct QueryParams {
    #[serde(default)]
    pub sql: Option<String>,

    #[serde(default)]
    pub command: Option<Vec<String>>,

    #[serde(default)]
    pub params: Vec<Value>,

    #[serde(default)]
    pub limit: Option<usize>,

    #[serde(default)]
    pub max_bytes: Option<usize>,

    #[serde(default)]
    pub offset: u64,

    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct QueryResult {
    pub kind: String,
    pub effect: Effect,

    #[serde(flatten)]
    pub rows: Rows,

    pub duration_ms: u64,
}

impl DbQuery {
    pub(super) fn new(databases: Databases) -> Self {
        Self { databases }
    }
}

impl Action for DbQuery {
    const ID: &'static str = "db.query";
    const TITLE: &'static str = "Run a statement or a redis command on a database";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = QueryParams;
    type Output = QueryResult;

    fn check(&self, target: Option<&ResourceRef>, params: &QueryParams) -> Result<(), ActionError> {
        let engine = self.databases.check_target(target)?;

        params.check_limits()?;
        params.check_statement(engine)
    }

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut QueryParams,
    ) -> Result<Option<Value>, ActionError> {
        let Some(engine) = self.databases.engine(target) else {
            return Ok(None);
        };

        let mut shown = serde_json::to_value(&*params)
            .map_err(|error| ActionError::Failed(error.to_string()))?;
        let classified = params.classification(engine);

        if let Some(fields) = shown.as_object_mut() {
            fields.retain(|_, value| !value.is_null());
            fields.insert("judged".to_owned(), json!(classified));
        }

        Ok(Some(shown))
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &QueryParams) -> Risk {
        let Some(engine) = self.databases.engine(target) else {
            return Risk::Privileged;
        };

        self.databases
            .risk(target, params.classification(engine).effect)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &QueryParams) -> Option<String> {
        self.databases.destination(target)
    }

    fn recorded_params(&self, params: &Value) -> Value {
        recorded_statement(params)
    }

    fn summary(&self, output: &QueryResult) -> Value {
        json!({
            "kind": output.kind,
            "effect": output.effect,
            "rows": output.rows.count(),
            "affected": output.rows.affected,
            "more": output.rows.more,
            "bytes": output.rows.bytes,
            "duration_ms": output.duration_ms,
        })
    }

    async fn run(
        &self,
        context: &Context,
        params: QueryParams,
    ) -> Result<QueryResult, ActionError> {
        let engine = self
            .databases
            .engine(context.target())
            .ok_or_else(|| ActionError::failed("the target is no longer a database"))?;
        let classified = params.classification(engine);
        let access = if classified.is_read() {
            Access::ReadOnly
        } else {
            Access::ReadWrite
        };

        let page = params.page();
        let started = Instant::now();

        let rows = self
            .databases
            .with(context, params.timeout(), async |database| {
                match (&params.command, &params.sql) {
                    (Some(words), _) => database.command(words, page).await,
                    (None, sql) => {
                        let query = Query {
                            text: sql.as_deref().unwrap_or_default(),
                            params: &params.params,
                            access,
                            page,
                        };
                        database.query(&query).await
                    }
                }
            })
            .await?;

        Ok(QueryResult {
            kind: classified.kind,
            effect: classified.effect,
            rows,
            duration_ms: millis(started),
        })
    }
}

impl QueryParams {
    pub(crate) fn classification(&self, engine: Engine) -> Classification {
        match (&self.command, &self.sql) {
            (Some(words), _) => Classification::of_command(words),
            (None, Some(sql)) => Classification::of_sql(engine, sql),
            (None, None) => Classification::of_command::<String>(&[]),
        }
    }

    fn check_statement(&self, engine: Engine) -> Result<(), ActionError> {
        match (&self.sql, &self.command) {
            (Some(sql), None) if sql.trim().is_empty() => invalid("sql is empty"),
            (None, Some(words)) if words.is_empty() => invalid("command is empty"),
            (None, Some(_)) if !self.params.is_empty() => {
                invalid("a redis command carries its values in the command itself, not in params")
            }
            (Some(_), None) if engine == Engine::Redis => {
                invalid("redis takes a command, such as [\"GET\", \"user:1\"], not sql")
            }
            (None, Some(_)) if engine.speaks_sql() => {
                invalid("this database takes sql, not a command")
            }
            (Some(_), None) | (None, Some(_)) => Ok(()),
            _ => invalid("give either sql or command"),
        }
    }

    fn check_limits(&self) -> Result<(), ActionError> {
        if self
            .limit
            .is_some_and(|limit| limit == 0 || limit > MOST_ROWS)
        {
            return invalid(&format!("limit must be between 1 and {MOST_ROWS}"));
        }

        if self
            .max_bytes
            .is_some_and(|bytes| bytes == 0 || bytes > MOST_BYTES)
        {
            return invalid(&format!("max_bytes must be between 1 and {MOST_BYTES}"));
        }

        if self
            .timeout_seconds
            .is_some_and(|seconds| seconds == 0 || seconds > LONGEST_TIMEOUT)
        {
            return invalid(&format!(
                "timeout_seconds must be between 1 and {LONGEST_TIMEOUT}"
            ));
        }

        Ok(())
    }

    fn page(&self) -> Page {
        Page::new(
            self.limit.unwrap_or(DEFAULT_ROWS),
            self.max_bytes.unwrap_or(DEFAULT_BYTES),
        )
        .starting_at(self.offset)
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT))
    }
}

pub(super) fn millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn invalid(reason: &str) -> Result<(), ActionError> {
    Err(ActionError::InvalidParams(reason.to_owned()))
}
