mod catalog;
mod cells;
mod error;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use rusqlite::types::Value as SqliteValue;
use rusqlite::{Connection, InterruptHandle, OpenFlags, Row, params_from_iter};
use serde_json::{Value, json};
use tokio::task;

use self::catalog::Catalog;
use self::cells::{from_json, to_json};
use crate::driver::{Answer, Driver};
use crate::paging::Collector;
use crate::{Access, DbError, Engine, Place, Query, Rows, Schema, Server, TableName};

const BUSY_LIMIT: Duration = Duration::from_secs(5);
const VERSION: &str = "SELECT sqlite_version()";
const BEGIN: &str = "BEGIN DEFERRED";
const ROLLBACK: &str = "ROLLBACK";

pub(crate) struct Sqlite {
    path: PathBuf,
    open: Mutex<Option<Opened>>,
}

struct Opened {
    access: Access,
    connection: Arc<Mutex<Connection>>,
    interrupt: InterruptHandle,
}

impl Sqlite {
    pub(crate) fn new(server: &Server) -> Result<Self, DbError> {
        let Place::File(path) = server.place() else {
            return Err(DbError::InvalidInput(
                "sqlite needs a file, not a host".to_owned(),
            ));
        };

        Ok(Self {
            path: path.clone(),
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
        let params: Vec<_> = query.params.iter().map(from_json).collect();
        let (access, page) = (query.access, query.page);

        self.with(access, move |connection| {
            let mut statement = connection.prepare(&text)?;
            let columns = statement
                .column_names()
                .into_iter()
                .map(str::to_owned)
                .collect();
            let mut rows = statement.query(params_from_iter(params))?;

            let mut collector = Collector::new(page);

            while let Some(row) = rows.next()? {
                if collector.wants_more() {
                    collector.offer(cells(row));
                } else if access.is_read_only() {
                    break;
                }
            }

            drop(rows);
            let affected = (!access.is_read_only()).then(|| connection.changes());

            Ok(collector.finish(columns, affected))
        })
        .await
    }

    async fn plan(&self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let text = format!("EXPLAIN QUERY PLAN {text}");
        let params: Vec<_> = params.iter().map(from_json).collect();

        self.with(Access::ReadOnly, move |connection| {
            let mut statement = connection.prepare(&text)?;
            let steps = statement
                .query_map(params_from_iter(params), |row| {
                    let (id, parent, _unused, detail): (i64, i64, SqliteValue, String) =
                        row.try_into()?;
                    Ok(json!({ "id": id, "parent": parent, "detail": detail }))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Value::Array(steps))
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
            && opened.access == access
        {
            return Ok(Arc::clone(&opened.connection));
        }

        let connection = Self::open_file(&self.path, access)?;
        let interrupt = connection.get_interrupt_handle();
        let connection = Arc::new(Mutex::new(connection));

        *open = Some(Opened {
            access,
            connection: Arc::clone(&connection),
            interrupt,
        });

        Ok(connection)
    }

    fn open_file(path: &Path, access: Access) -> Result<Connection, DbError> {
        let flags = match access {
            Access::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
            Access::ReadWrite => OpenFlags::SQLITE_OPEN_READ_WRITE,
        } | OpenFlags::SQLITE_OPEN_NO_MUTEX;

        let connection = Connection::open_with_flags(path, flags).map_err(|error| {
            DbError::Connect(format!("cannot open {}: {error}", path.display()))
        })?;

        connection.busy_timeout(BUSY_LIMIT)?;

        if access.is_read_only() {
            connection.pragma_update(None, "query_only", true)?;
        }

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
            opened.interrupt.interrupt();
        }
    }
}

fn cells(row: &Row<'_>) -> Vec<Value> {
    (0..row.as_ref().column_count())
        .map(|index| row.get_ref(index).map_or(Value::Null, to_json))
        .collect()
}
