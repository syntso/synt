use futures_util::TryStreamExt;
use mysql_async::prelude::Queryable;
use mysql_async::{Conn, Opts, OptsBuilder, Params, QueryResult, Row, SslOpts, Value as Cell};
use serde_json::Value;
use tokio::time;

use super::catalog;
use super::decode::decode;
use super::failure::{failure, lacks_tls};
use crate::rows::Collector;
use crate::schema::{Column, ForeignKeyPart, IndexPart, Schema, TableName};
use crate::tls::SystemRoots;
use crate::{DbError, Place, Query, Rows, Server, TlsMode};

const READ_ONLY_SESSION: &str = "SET SESSION TRANSACTION READ ONLY";
const READ_ONLY_TRANSACTION: &str = "START TRANSACTION READ ONLY";
const ROLLBACK: &str = "ROLLBACK";

pub(crate) struct Mysql {
    connection: Conn,
}

impl Mysql {
    pub(crate) async fn connect(server: &Server) -> Result<Self, DbError> {
        let limit = server.connect_limit();

        let connecting = async {
            match Conn::new(options(server, server.tls())?).await {
                Err(error) if server.tls() == TlsMode::Prefer && lacks_tls(&error) => {
                    Conn::new(options(server, TlsMode::Disable)?)
                        .await
                        .map_err(|error| failure(&error))
                }
                connected => connected.map_err(|error| failure(&error)),
            }
        };

        let connection = time::timeout(limit, connecting)
            .await
            .map_err(|_| DbError::Timeout(limit))??;

        Ok(Self { connection })
    }

    pub(crate) async fn ping(&mut self) -> Result<String, DbError> {
        let version: Option<String> = self
            .connection
            .query_first("SELECT VERSION()")
            .await
            .map_err(|error| failure(&error))?;

        Ok(version.unwrap_or_default())
    }

    pub(crate) async fn query(&mut self, query: &Query<'_>) -> Result<Rows, DbError> {
        if !query.access.is_read_only() {
            return run(&mut self.connection, query).await;
        }

        self.start_read_only().await?;
        let rows = run(&mut self.connection, query).await;
        self.end_read_only().await;

        rows
    }

    pub(crate) async fn explain(&mut self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let explained = format!("EXPLAIN FORMAT=JSON {text}");

        self.start_read_only().await?;
        let plan = first_cell(&mut self.connection, &explained, params).await;
        self.end_read_only().await;

        let plan = plan?;
        let parsed = plan
            .as_str()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or(plan);

        Ok(parsed)
    }

    pub(crate) async fn tables(&mut self) -> Result<Vec<TableName>, DbError> {
        self.start_read_only().await?;
        let names = table_names(&mut self.connection).await;
        self.end_read_only().await;

        names
    }

    pub(crate) async fn schema(&mut self) -> Result<Schema, DbError> {
        self.start_read_only().await?;
        let schema = read_schema(&mut self.connection).await;
        self.end_read_only().await;

        schema
    }

    pub(crate) async fn close(self) {
        let _closed = self.connection.disconnect().await;
    }

    async fn start_read_only(&mut self) -> Result<(), DbError> {
        for statement in [READ_ONLY_SESSION, READ_ONLY_TRANSACTION] {
            self.connection
                .query_drop(statement)
                .await
                .map_err(|error| failure(&error))?;
        }

        Ok(())
    }

    async fn end_read_only(&mut self) {
        let _ended = self.connection.query_drop(ROLLBACK).await;
    }
}

fn options(server: &Server, tls: TlsMode) -> Result<Opts, DbError> {
    let Place::Network {
        host,
        port,
        reached_at,
    } = server.place()
    else {
        return Err(DbError::InvalidInput(
            "mysql needs a host and a port, not a file".to_owned(),
        ));
    };

    let (dial_host, dial_port) = match reached_at {
        Some(address) => (address.ip().to_string(), address.port()),
        None => (host.clone(), *port),
    };

    let builder = OptsBuilder::default()
        .ip_or_hostname(dial_host)
        .tcp_port(dial_port)
        .prefer_socket(false)
        .user(server.user())
        .pass(server.password())
        .db_name(server.database())
        .ssl_opts(ssl_options(tls, host, reached_at.is_some()));

    Ok(builder.into())
}

fn ssl_options(tls: TlsMode, host: &str, tunneled: bool) -> Option<SslOpts> {
    match tls {
        TlsMode::Disable => None,
        TlsMode::Prefer | TlsMode::Require => Some(
            SslOpts::default()
                .with_danger_accept_invalid_certs(true)
                .with_danger_skip_domain_validation(true),
        ),
        TlsMode::VerifyFull => {
            let roots = SystemRoots::load().into_der().map(Into::into).collect();
            let options = SslOpts::default()
                .with_disable_built_in_roots(true)
                .with_root_certs(roots);

            Some(if tunneled {
                options.with_danger_tls_hostname_override(Some(host.to_owned()))
            } else {
                options
            })
        }
    }
}

async fn run(connection: &mut Conn, query: &Query<'_>) -> Result<Rows, DbError> {
    if query.params.is_empty() && !query.access.is_read_only() {
        let result = connection
            .query_iter(query.text)
            .await
            .map_err(|error| failure(&error))?;
        return collect(result, query).await;
    }

    let result = connection
        .exec_iter(query.text, params(query.params))
        .await
        .map_err(|error| failure(&error))?;

    collect(result, query).await
}

async fn collect<P>(mut result: QueryResult<'_, '_, P>, query: &Query<'_>) -> Result<Rows, DbError>
where
    P: mysql_async::prelude::Protocol + Unpin,
{
    let columns = result.columns().unwrap_or_default();
    let names = columns
        .iter()
        .map(|column| column.name_str().into_owned())
        .collect();
    let mut collector = Collector::new(query.page);

    if let Some(mut stream) = result
        .stream::<Row>()
        .await
        .map_err(|error| failure(&error))?
    {
        while let Some(row) = stream.try_next().await.map_err(|error| failure(&error))? {
            if collector.wants_more() {
                collector.offer(cells(row));
            } else if query.access.is_read_only() {
                break;
            }
        }
    }

    let affected = (!query.access.is_read_only()).then(|| result.affected_rows());
    result
        .drop_result()
        .await
        .map_err(|error| failure(&error))?;

    Ok(collector.finish(names, affected))
}

fn cells(row: Row) -> Vec<Value> {
    let columns = row.columns();

    row.unwrap()
        .into_iter()
        .zip(columns.iter())
        .map(|(cell, column)| decode(column, cell))
        .collect()
}

fn params(values: &[Value]) -> Params {
    Params::Positional(values.iter().map(cell).collect())
}

fn cell(value: &Value) -> Cell {
    match value {
        Value::Null => Cell::NULL,
        Value::Bool(flag) => Cell::Int(i64::from(*flag)),
        Value::Number(number) => number
            .as_i64()
            .map(Cell::Int)
            .or_else(|| number.as_u64().map(Cell::UInt))
            .or_else(|| number.as_f64().map(Cell::Double))
            .unwrap_or(Cell::NULL),
        Value::String(text) => Cell::Bytes(text.clone().into_bytes()),
        other => Cell::Bytes(other.to_string().into_bytes()),
    }
}

async fn first_cell(connection: &mut Conn, text: &str, params: &[Value]) -> Result<Value, DbError> {
    let query = Query {
        text,
        params,
        access: crate::Access::ReadOnly,
        page: crate::Page::new(1, usize::MAX),
    };

    let rows = run(connection, &query).await?;

    Ok(rows
        .rows
        .into_iter()
        .next()
        .and_then(|row| row.into_iter().next())
        .unwrap_or(Value::Null))
}

async fn catalog_rows(connection: &mut Conn, sql: &str) -> Result<Vec<Row>, DbError> {
    connection.query(sql).await.map_err(|error| failure(&error))
}

async fn table_names(connection: &mut Conn) -> Result<Vec<TableName>, DbError> {
    let rows = catalog_rows(connection, &catalog::tables()).await?;

    rows.iter()
        .map(|row| {
            Ok(TableName {
                schema: Some(text(row, 0)?),
                name: text(row, 1)?,
                kind: text(row, 2)?,
            })
        })
        .collect()
}

async fn read_schema(connection: &mut Conn) -> Result<Schema, DbError> {
    let mut schema = Schema::of(table_names(connection).await?);

    for row in catalog_rows(connection, &catalog::columns()).await? {
        let column = Column {
            name: text(&row, 2)?,
            data_type: text(&row, 3)?,
            nullable: flag(&row, 4)?,
            default: optional_text(&row, 5),
        };
        schema.add_column(Some(&text(&row, 0)?), &text(&row, 1)?, column);
    }

    for row in catalog_rows(connection, &catalog::indexes()).await? {
        let part = IndexPart {
            index: text(&row, 2)?,
            column: text(&row, 3)?,
            unique: flag(&row, 4)?,
            primary: flag(&row, 5)?,
        };
        schema.add_index_part(Some(&text(&row, 0)?), &text(&row, 1)?, part);
    }

    for row in catalog_rows(connection, &catalog::foreign_keys()).await? {
        let name = text(&row, 2)?;
        let part = ForeignKeyPart {
            key: name.clone(),
            name: Some(name),
            column: text(&row, 3)?,
            referenced_schema: optional_text(&row, 4),
            referenced_table: text(&row, 5)?,
            referenced_column: text(&row, 6)?,
        };
        schema.add_foreign_key_part(Some(&text(&row, 0)?), &text(&row, 1)?, part);
    }

    Ok(schema)
}

fn text(row: &Row, index: usize) -> Result<String, DbError> {
    optional_text(row, index).ok_or_else(|| unexpected(index))
}

fn optional_text(row: &Row, index: usize) -> Option<String> {
    match row.as_ref(index) {
        None | Some(Cell::NULL) => None,
        Some(Cell::Bytes(bytes)) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Some(other) => Some(other.as_sql(true)),
    }
}

fn flag(row: &Row, index: usize) -> Result<bool, DbError> {
    let text = text(row, index)?;

    Ok(text != "0")
}

fn unexpected(index: usize) -> DbError {
    DbError::Lost(format!(
        "the catalog answered with an unexpected value in column {index}"
    ))
}
