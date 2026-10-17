mod fields;
mod statement;

use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_db::{Effect, Page, Rows, StatementKind};
use synt_journal::ResourceRef;
use synt_proto::Millis;

use self::fields::QueryFields;
use self::statement::Statement;
use super::Databases;
use super::error::DbActionError;
use super::recorded::recorded_statement;

use crate::CoreError;

const DEFAULT_ROWS: usize = 100;
const MOST_ROWS: usize = 10_000;
const DEFAULT_BYTES: usize = 1024 * 1024;
const MOST_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_TIMEOUT: u64 = 60;
const LONGEST_TIMEOUT: u64 = 3600;
const JUDGED: &str = "judged";

#[derive(Debug, Clone)]
pub(crate) struct DbQuery {
    databases: Databases,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(try_from = "QueryFields", into = "QueryFields")]
pub(crate) struct QueryParams {
    statement: Statement,
    limit: Option<usize>,
    max_bytes: Option<usize>,
    offset: u64,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct QueryResult {
    pub kind: StatementKind,
    pub effect: Effect,

    #[serde(flatten)]
    pub rows: Rows,

    pub duration_ms: Millis,
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

        Ok(params.statement.check(engine)?)
    }

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut QueryParams,
    ) -> Result<Option<Value>, ActionError> {
        let Some(engine) = self.databases.engine(target) else {
            return Ok(None);
        };

        let judged = params.statement.classification(engine);
        let mut shown = serde_json::to_value(&*params).map_err(CoreError::from)?;

        if let Some(fields) = shown.as_object_mut() {
            fields.retain(|_, value| !value.is_null());
            fields.insert(JUDGED.to_owned(), json!(judged));
        }

        Ok(Some(shown))
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &QueryParams) -> Risk {
        let Some(engine) = self.databases.engine(target) else {
            return Risk::Privileged;
        };

        let judged = params.statement.classification(engine);

        self.databases.risk(target, judged.effect)
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
        let engine = self.databases.current_engine(context)?;
        let judged = params.statement.classification(engine);
        let access = judged.access();
        let page = params.page();
        let started = Instant::now();

        let rows = self
            .databases
            .with(context, params.timeout(), async |database| {
                params.statement.run(database, access, page).await
            })
            .await?;

        Ok(QueryResult {
            kind: judged.kind,
            effect: judged.effect,
            rows,
            duration_ms: Millis::from(started.elapsed()),
        })
    }
}

impl QueryParams {
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

impl TryFrom<QueryFields> for QueryParams {
    type Error = DbActionError;

    fn try_from(fields: QueryFields) -> Result<Self, DbActionError> {
        within("limit", fields.limit, MOST_ROWS)?;
        within("max_bytes", fields.max_bytes, MOST_BYTES)?;
        within("timeout_seconds", fields.timeout_seconds, LONGEST_TIMEOUT)?;

        let statement = Statement::new(fields.sql, fields.command, fields.params)?;

        Ok(Self {
            statement,
            limit: fields.limit,
            max_bytes: fields.max_bytes,
            offset: fields.offset,
            timeout_seconds: fields.timeout_seconds,
        })
    }
}

fn within<N>(field: &'static str, given: Option<N>, most: N) -> Result<(), DbActionError>
where
    N: Copy + PartialOrd + Default + TryInto<u64>,
{
    let Some(given) = given else {
        return Ok(());
    };

    if given > N::default() && given <= most {
        return Ok(());
    }

    Err(DbActionError::OutOfRange {
        field,
        most: most.try_into().unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::QueryParams;

    fn parse(params: Value) -> Result<QueryParams, String> {
        serde_json::from_value(params).map_err(|error| error.to_string())
    }

    #[test]
    fn limits_outside_their_range_are_refused() {
        for params in [
            json!({ "sql": "SELECT 1", "limit": 0 }),
            json!({ "sql": "SELECT 1", "limit": 10_001 }),
            json!({ "sql": "SELECT 1", "max_bytes": 0 }),
            json!({ "sql": "SELECT 1", "timeout_seconds": 3601 }),
        ] {
            assert!(parse(params.clone()).is_err(), "{params}");
        }
    }

    #[test]
    fn a_range_error_names_the_field_and_its_bounds() {
        let error = parse(json!({ "sql": "SELECT 1", "limit": 0 })).unwrap_err();

        assert_eq!(error, "limit must be between 1 and 10000");
    }

    #[test]
    fn params_are_shown_as_they_were_given() {
        let given = json!({ "sql": "SELECT $1", "params": [7], "limit": 5 });

        let shown = serde_json::to_value(parse(given).expect("params")).expect("shown");

        assert_eq!(
            shown,
            json!({
                "sql": "SELECT $1",
                "command": null,
                "params": [7],
                "limit": 5,
                "max_bytes": null,
                "offset": 0,
                "timeout_seconds": null,
            })
        );
    }

    #[test]
    fn unknown_fields_are_refused() {
        assert!(parse(json!({ "sql": "SELECT 1", "rows": 5 })).is_err());
    }
}
