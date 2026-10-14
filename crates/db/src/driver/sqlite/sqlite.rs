use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, InterruptHandle, OpenFlags, Row, params_from_iter};
use serde_json::{Value, json};

use super::catalog;
use super::cells::{from_json, to_json};
use super::failure::failure;
use crate::catalog::{Column, ForeignKeyPart, IndexPart, Schema, TableName};
use crate::paging::Collector;
use crate::{Access, DbError, Place, Query, Rows, Server};

const BUSY_LIMIT: Duration = Duration::from_secs(5);

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

    pub(crate) async fn ping(&self) -> Result<String, DbError> {
        self.with(Access::ReadOnly, |connection| {
            connection
                .query_row("SELECT sqlite_version()", [], |row| row.get(0))
                .map_err(|error| failure(&error))
        })
        .await
    }

    pub(crate) async fn query(&self, query: &Query<'_>) -> Result<Rows, DbError> {
        let text = query.text.to_owned();
        let params: Vec<_> = query.params.iter().map(from_json).collect();
        let (access, page) = (query.access, query.page);

        self.with(access, move |connection| {
            let mut statement = connection.prepare(&text).map_err(|error| failure(&error))?;
            let columns = statement
                .column_names()
                .into_iter()
                .map(str::to_owned)
                .collect();
            let mut rows = statement
                .query(params_from_iter(params))
                .map_err(|error| failure(&error))?;

            let mut collector = Collector::new(page);

            while let Some(row) = rows.next().map_err(|error| failure(&error))? {
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

    pub(crate) async fn explain(&self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let text = format!("EXPLAIN QUERY PLAN {text}");
        let params: Vec<_> = params.iter().map(from_json).collect();

        self.with(Access::ReadOnly, move |connection| {
            let mut statement = connection.prepare(&text).map_err(|error| failure(&error))?;
            let steps = statement
                .query_map(params_from_iter(params), |row| {
                    Ok(json!({
                        "id": row.get::<_, i64>(0)?,
                        "parent": row.get::<_, i64>(1)?,
                        "detail": row.get::<_, String>(3)?,
                    }))
                })
                .map_err(|error| failure(&error))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| failure(&error))?;

            Ok(Value::Array(steps))
        })
        .await
    }

    pub(crate) async fn tables(&self) -> Result<Vec<TableName>, DbError> {
        self.with(Access::ReadOnly, table_names).await
    }

    pub(crate) async fn schema(&self) -> Result<Schema, DbError> {
        self.with(Access::ReadOnly, read_schema).await
    }

    async fn with<T, Work>(&self, access: Access, work: Work) -> Result<T, DbError>
    where
        T: Send + 'static,
        Work: FnOnce(&Connection) -> Result<T, DbError> + Send + 'static,
    {
        let connection = self.connection(access)?;

        tokio::task::spawn_blocking(move || {
            let connection = connection.lock().unwrap_or_else(PoisonError::into_inner);
            in_transaction(&connection, access, work)
        })
        .await
        .map_err(|error| DbError::Lost(error.to_string()))?
    }

    fn connection(&self, access: Access) -> Result<Arc<Mutex<Connection>>, DbError> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);

        if let Some(opened) = open.as_ref()
            && opened.access == access
        {
            return Ok(Arc::clone(&opened.connection));
        }

        let connection = connect(&self.path, access)?;
        let interrupt = connection.get_interrupt_handle();
        let connection = Arc::new(Mutex::new(connection));

        *open = Some(Opened {
            access,
            connection: Arc::clone(&connection),
            interrupt,
        });

        Ok(connection)
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

fn connect(path: &Path, access: Access) -> Result<Connection, DbError> {
    let flags = match access {
        Access::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        Access::ReadWrite => OpenFlags::SQLITE_OPEN_READ_WRITE,
    } | OpenFlags::SQLITE_OPEN_NO_MUTEX;

    let connection = Connection::open_with_flags(path, flags)
        .map_err(|error| DbError::Connect(format!("cannot open {}: {error}", path.display())))?;

    connection
        .busy_timeout(BUSY_LIMIT)
        .map_err(|error| failure(&error))?;

    if access.is_read_only() {
        connection
            .pragma_update(None, "query_only", true)
            .map_err(|error| failure(&error))?;
    }

    Ok(connection)
}

fn in_transaction<T>(
    connection: &Connection,
    access: Access,
    work: impl FnOnce(&Connection) -> Result<T, DbError>,
) -> Result<T, DbError> {
    if !access.is_read_only() {
        return work(connection);
    }

    connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(|error| failure(&error))?;

    let done = work(connection);
    let _ended = connection.execute_batch("ROLLBACK");

    done
}

fn cells(row: &Row<'_>) -> Vec<Value> {
    (0..row.as_ref().column_count())
        .map(|index| row.get_ref(index).map_or(Value::Null, to_json))
        .collect()
}

fn table_names(connection: &Connection) -> Result<Vec<TableName>, DbError> {
    catalog_rows(connection, &catalog::tables(), |row| {
        Ok(TableName {
            schema: None,
            name: row.get(0)?,
            kind: row.get(1)?,
        })
    })
}

fn read_schema(connection: &Connection) -> Result<Schema, DbError> {
    let mut schema = Schema::of(table_names(connection)?);
    let mut keys = Vec::new();

    let columns = catalog_rows(connection, &catalog::columns(), |row| {
        let column = Column {
            name: row.get(1)?,
            data_type: row.get(2)?,
            nullable: row.get::<_, i64>(3)? == 0,
            default: row.get(4)?,
        };
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(5)?, column))
    })?;

    for (table, position, column) in columns {
        if position > 0 {
            keys.push((table.clone(), position, column.name.clone()));
        }
        schema.add_column(None, &table, column);
    }

    keys.sort();
    for (table, _, column) in keys {
        schema.add_primary_key_column(None, &table, column);
    }

    add_indexes(connection, &mut schema)?;
    add_foreign_keys(connection, &mut schema)?;

    Ok(schema)
}

fn add_indexes(connection: &Connection, schema: &mut Schema) -> Result<(), DbError> {
    let parts = catalog_rows(connection, &catalog::indexes(), |row| {
        let part = IndexPart {
            index: row.get(1)?,
            column: row.get(2)?,
            unique: row.get(3)?,
            primary: row.get(4)?,
        };
        Ok((row.get::<_, String>(0)?, part))
    })?;

    for (table, part) in parts {
        schema.add_index_part(None, &table, part);
    }

    Ok(())
}

fn add_foreign_keys(connection: &Connection, schema: &mut Schema) -> Result<(), DbError> {
    let parts = catalog_rows(connection, &catalog::foreign_keys(), |row| {
        let part = ForeignKeyPart {
            key: row.get::<_, i64>(1)?.to_string(),
            name: None,
            column: row.get(2)?,
            referenced_schema: None,
            referenced_table: row.get(3)?,
            referenced_column: row.get(4)?,
        };
        Ok((row.get::<_, String>(0)?, part))
    })?;

    for (table, part) in parts {
        schema.add_foreign_key_part(None, &table, part);
    }

    Ok(())
}

fn catalog_rows<T>(
    connection: &Connection,
    sql: &str,
    read: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, DbError> {
    let mut statement = connection.prepare(sql).map_err(|error| failure(&error))?;

    statement
        .query_map([], read)
        .map_err(|error| failure(&error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| failure(&error))
}
