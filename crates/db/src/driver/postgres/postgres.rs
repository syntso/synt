use std::net::IpAddr;

use futures_util::{TryStreamExt, pin_mut};
use serde_json::Value;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_postgres::config::SslMode;
use tokio_postgres::types::ToSql;
use tokio_postgres::{Client, Config, GenericClient, NoTls, Row, Statement};
use tokio_postgres_rustls::MakeRustlsConnect;

use super::catalog;
use super::decode::decode;
use super::failure::failure;
use super::raw_cell::RawCell;
use super::text_param::TextParam;
use crate::rows::Collector;
use crate::schema::{Column, ForeignKeyPart, IndexPart, Schema, TableName};
use crate::tls::client_config;
use crate::{DbError, Place, Query, Rows, Server, TlsMode};

const APPLICATION: &str = "synt";

pub(crate) struct Postgres {
    client: Client,
    driver: JoinHandle<()>,
}

impl Postgres {
    pub(crate) async fn connect(server: &Server) -> Result<Self, DbError> {
        let config = config(server)?;
        let tls = client_config(server.tls())?;
        let limit = server.connect_limit();

        let connecting = async {
            match tls {
                None => {
                    let (client, connection) = config.connect(NoTls).await?;
                    Ok((client, tokio::spawn(drive(connection))))
                }
                Some(tls) => {
                    let maker = MakeRustlsConnect::new((*tls).clone());
                    let (client, connection) = config.connect(maker).await?;
                    Ok((client, tokio::spawn(drive(connection))))
                }
            }
        };

        let (client, driver) = time::timeout(limit, connecting)
            .await
            .map_err(|_| DbError::Timeout(limit))?
            .map_err(|error| failure(&error))?;

        Ok(Self { client, driver })
    }

    pub(crate) async fn ping(&self) -> Result<String, DbError> {
        let row = self
            .client
            .query_one("SELECT current_setting('server_version')", &[])
            .await
            .map_err(|error| failure(&error))?;

        text(&row, 0)
    }

    pub(crate) async fn query(&mut self, query: &Query<'_>) -> Result<Rows, DbError> {
        if !query.access.is_read_only() {
            return run(&self.client, query).await;
        }

        let transaction = self.read_only().await?;
        let rows = run(&transaction, query).await;
        let _ended = transaction.rollback().await;

        rows
    }

    pub(crate) async fn explain(&mut self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let transaction = self.read_only().await?;
        let explained = format!("EXPLAIN (FORMAT JSON) {text}");

        let statement = prepare(&transaction, &explained, params).await?;
        let params = text_params(params);
        let rows = transaction
            .query(&statement, &borrowed(&params))
            .await
            .map_err(|error| failure(&error))?;
        let _ended = transaction.rollback().await;

        let plan = rows.first().map_or(Value::Null, |row| cell(row, 0));

        Ok(plan)
    }

    pub(crate) async fn tables(&mut self) -> Result<Vec<TableName>, DbError> {
        let transaction = self.read_only().await?;
        let names = table_names(&transaction).await;
        let _ended = transaction.rollback().await;

        names
    }

    pub(crate) async fn schema(&mut self) -> Result<Schema, DbError> {
        let transaction = self.read_only().await?;
        let schema = read_schema(&transaction).await;
        let _ended = transaction.rollback().await;

        schema
    }

    async fn read_only(&mut self) -> Result<tokio_postgres::Transaction<'_>, DbError> {
        self.client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(|error| failure(&error))
    }
}

impl Drop for Postgres {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

fn config(server: &Server) -> Result<Config, DbError> {
    let Place::Network {
        host,
        port,
        reached_at,
    } = server.place()
    else {
        return Err(DbError::InvalidInput(
            "postgres needs a host and a port, not a file".to_owned(),
        ));
    };

    let mut config = Config::new();
    config
        .host(host)
        .port(reached_at.map_or(*port, |address| address.port()))
        .application_name(APPLICATION)
        .connect_timeout(server.connect_limit())
        .ssl_mode(ssl_mode(server.tls()));

    if let Some(address) = reached_at {
        config.hostaddr(address.ip());
    } else if let Ok(address) = host.parse::<IpAddr>() {
        config.hostaddr(address);
    }

    if let Some(database) = server.database() {
        config.dbname(database);
    }
    if let Some(user) = server.user() {
        config.user(user);
    }
    if let Some(password) = server.password() {
        config.password(password);
    }

    Ok(config)
}

fn ssl_mode(tls: TlsMode) -> SslMode {
    match tls {
        TlsMode::Disable => SslMode::Disable,
        TlsMode::Prefer => SslMode::Prefer,
        TlsMode::Require | TlsMode::VerifyFull => SslMode::Require,
    }
}

async fn drive<S, T>(connection: tokio_postgres::Connection<S, T>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let _ended = connection.await;
}

async fn run(client: &impl GenericClient, query: &Query<'_>) -> Result<Rows, DbError> {
    let statement = prepare(client, query.text, query.params).await?;
    let params = text_params(query.params);
    let columns = statement
        .columns()
        .iter()
        .map(|column| column.name().to_owned())
        .collect();

    let stream = client
        .query_raw(&statement, borrowed(&params))
        .await
        .map_err(|error| failure(&error))?;
    pin_mut!(stream);

    let mut collector = Collector::new(query.page);

    while let Some(row) = stream.try_next().await.map_err(|error| failure(&error))? {
        if collector.wants_more() {
            collector.offer(cells(&row));
        } else if query.access.is_read_only() {
            break;
        }
    }

    let affected = if query.access.is_read_only() {
        None
    } else {
        stream.rows_affected()
    };

    Ok(collector.finish(columns, affected))
}

async fn prepare(
    client: &impl GenericClient,
    text: &str,
    params: &[Value],
) -> Result<Statement, DbError> {
    let statement = client
        .prepare(text)
        .await
        .map_err(|error| failure(&error))?;
    let expected = statement.params().len();

    if expected != params.len() {
        return Err(DbError::InvalidInput(format!(
            "the statement takes {expected} parameter(s), {} given",
            params.len()
        )));
    }

    Ok(statement)
}

fn text_params(params: &[Value]) -> Vec<TextParam> {
    params.iter().map(TextParam::from_json).collect()
}

fn borrowed(params: &[TextParam]) -> Vec<&(dyn ToSql + Sync)> {
    params
        .iter()
        .map(|param| param as &(dyn ToSql + Sync))
        .collect()
}

fn cells(row: &Row) -> Vec<Value> {
    (0..row.len()).map(|index| cell(row, index)).collect()
}

fn cell(row: &Row, index: usize) -> Value {
    let kind = row.columns()[index].type_();

    match row.try_get::<_, RawCell<'_>>(index) {
        Ok(RawCell(bytes)) => decode(kind, bytes),
        Err(_) => Value::Null,
    }
}

async fn table_names(client: &impl GenericClient) -> Result<Vec<TableName>, DbError> {
    let rows = catalog_rows(client, &catalog::tables()).await?;

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

async fn read_schema(client: &impl GenericClient) -> Result<Schema, DbError> {
    let mut schema = Schema::of(table_names(client).await?);

    for row in catalog_rows(client, &catalog::columns()).await? {
        let column = Column {
            name: text(&row, 2)?,
            data_type: text(&row, 3)?,
            nullable: flag(&row, 4)?,
            default: optional_text(&row, 5)?,
        };
        schema.add_column(Some(&text(&row, 0)?), &text(&row, 1)?, column);
    }

    for row in catalog_rows(client, &catalog::indexes()).await? {
        let part = IndexPart {
            index: text(&row, 2)?,
            column: text(&row, 3)?,
            unique: flag(&row, 4)?,
            primary: flag(&row, 5)?,
        };
        schema.add_index_part(Some(&text(&row, 0)?), &text(&row, 1)?, part);
    }

    for row in catalog_rows(client, &catalog::foreign_keys()).await? {
        let name = text(&row, 2)?;
        let part = ForeignKeyPart {
            key: name.clone(),
            name: Some(name),
            column: text(&row, 3)?,
            referenced_schema: Some(text(&row, 4)?),
            referenced_table: text(&row, 5)?,
            referenced_column: text(&row, 6)?,
        };
        schema.add_foreign_key_part(Some(&text(&row, 0)?), &text(&row, 1)?, part);
    }

    Ok(schema)
}

async fn catalog_rows(client: &impl GenericClient, sql: &str) -> Result<Vec<Row>, DbError> {
    client
        .query(sql, &[])
        .await
        .map_err(|error| failure(&error))
}

fn text(row: &Row, index: usize) -> Result<String, DbError> {
    optional_text(row, index)?.ok_or_else(|| unexpected(index))
}

fn optional_text(row: &Row, index: usize) -> Result<Option<String>, DbError> {
    row.try_get(index).map_err(|_| unexpected(index))
}

fn flag(row: &Row, index: usize) -> Result<bool, DbError> {
    row.try_get(index).map_err(|_| unexpected(index))
}

fn unexpected(index: usize) -> DbError {
    DbError::Lost(format!(
        "the catalog answered with an unexpected value in column {index}"
    ))
}
