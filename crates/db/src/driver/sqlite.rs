mod catalog;
mod cell;
mod error;
mod opened;
mod param;
mod plan_step;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use rusqlite::{Connection, params_from_iter};
use serde_json::{Value, json};
use tokio::task;

use self::catalog::Catalog;
use self::cell::Cell;
use self::opened::Opened;
use self::param::Param;
use self::plan_step::PlanStep;
use crate::driver::{Answer, Driver};
use crate::paging::Collector;
use crate::{Access, DbError, Engine, Query, Rows, Schema, Server, TableName};

const VERSION: &str = "SELECT sqlite_version()";
const BEGIN: &str = "BEGIN DEFERRED";
const ROLLBACK: &str = "ROLLBACK";

pub(crate) struct Sqlite {
    path: PathBuf,
    open: Mutex<Option<Opened>>,
}

impl Sqlite {
    pub(crate) fn new(server: &Server) -> Result<Self, DbError> {
        let Some(path) = server.place().file() else {
            return Err(DbError::InvalidInput(
                "sqlite needs a file, not a host".to_owned(),
            ));
        };

        Ok(Self {
            path: path.to_owned(),
            open: Mutex::new(None),
        })
    }

    async fn version(&self) -> Result<String, DbError> {
        self.with(Access::ReadOnly, |connection| {
            Ok(connection.query_row(VERSION, [], |row| row.get(0))?)
        })
        .await
    }

    async fn run(&self, query: Query<'_>) -> Result<Rows, DbError> {
        let text = query.text.to_owned();
        let params: Vec<Param> = query.params.iter().map(Param::from).collect();
        let collector = Collector::new(query.page, query.access);

        self.with(query.access, move |connection| {
            Self::collect(connection, &text, params, collector)
        })
        .await
    }

    async fn plan(&self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let text = format!("EXPLAIN QUERY PLAN {text}");
        let params: Vec<Param> = params.iter().map(Param::from).collect();

        self.with(Access::ReadOnly, move |connection| {
            let mut statement = connection.prepare(&text)?;
            let steps = statement
                .query_map(params_from_iter(params), |row| PlanStep::try_from(row))?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(json!(steps))
        })
        .await
    }

    async fn table_names(&self) -> Result<Vec<TableName>, DbError> {
        self.with(Access::ReadOnly, |connection| {
            Catalog::new(connection).tables()
        })
        .await
    }

    async fn read_schema(&self) -> Result<Schema, DbError> {
        self.with(Access::ReadOnly, |connection| {
            Catalog::new(connection).schema()
        })
        .await
    }

    fn collect(
        connection: &Connection,
        text: &str,
        params: Vec<Param>,
        mut collector: Collector,
    ) -> Result<Rows, DbError> {
        let mut statement = connection.prepare(text)?;
        let columns = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();

        let mut rows = statement.query(params_from_iter(params))?;

        while let Some(row) = rows.next()? {
            if collector.take(|| Cell::row(row)).is_break() {
                break;
            }
        }

        drop(rows);

        Ok(collector.finish(columns, || Some(connection.changes())))
    }

    async fn with<T, Work>(&self, access: Access, work: Work) -> Result<T, DbError>
    where
        T: Send + 'static,
        Work: FnOnce(&Connection) -> Result<T, DbError> + Send + 'static,
    {
        let connection = self.connection(access)?;

        task::spawn_blocking(move || {
            let connection = connection.lock().unwrap_or_else(PoisonError::into_inner);
            Self::guarded(&connection, access, work)
        })
        .await
        .map_err(|error| DbError::Lost(error.to_string()))?
    }

    fn guarded<T>(
        connection: &Connection,
        access: Access,
        work: impl FnOnce(&Connection) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        if !access.is_read_only() {
            return work(connection);
        }

        connection.execute_batch(BEGIN)?;

        let done = work(connection);
        let _ended = connection.execute_batch(ROLLBACK);

        done
    }

    fn connection(&self, access: Access) -> Result<Arc<Mutex<Connection>>, DbError> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);

        if let Some(opened) = open.as_ref()
            && opened.serves(access)
        {
            return Ok(opened.connection());
        }

        let opened = Opened::open(&self.path, access)?;
        let connection = opened.connection();

        *open = Some(opened);

        Ok(connection)
    }
}

impl Driver for Sqlite {
    fn engine(&self) -> Engine {
        Engine::Sqlite
    }

    fn ping(&mut self) -> Answer<'_, String> {
        Box::pin(self.version())
    }

    fn query<'a>(&'a mut self, query: Query<'a>) -> Answer<'a, Rows> {
        Box::pin(self.run(query))
    }

    fn explain<'a>(&'a mut self, text: &'a str, params: &'a [Value]) -> Answer<'a, Value> {
        Box::pin(self.plan(text, params))
    }

    fn tables(&mut self) -> Answer<'_, Vec<TableName>> {
        Box::pin(self.table_names())
    }

    fn schema(&mut self) -> Answer<'_, Schema> {
        Box::pin(self.read_schema())
    }
}

impl Drop for Sqlite {
    fn drop(&mut self) {
        let open = self.open.get_mut().unwrap_or_else(PoisonError::into_inner);

        if let Some(opened) = open.as_ref() {
            opened.interrupt();
        }
    }
}
