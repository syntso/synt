mod catalog;
mod cell;
mod error;
mod options;
mod params;
mod results;

use mysql_async::prelude::Queryable;
use mysql_async::{Conn, DriverError, Error, Opts};
use serde_json::Value;

use self::catalog::Catalog;
use self::params::params;
use self::results::collect;
use crate::driver::{Answer, Driver};
use crate::{DbError, Engine, Query, Rows, Schema, Server, TableName, TlsMode};

const VERSION: &str = "SELECT VERSION()";
const READ_ONLY_SESSION: &str = "SET SESSION TRANSACTION READ ONLY";
const READ_ONLY_TRANSACTION: &str = "START TRANSACTION READ ONLY";
const ROLLBACK: &str = "ROLLBACK";

pub(crate) struct Mysql {
    connection: Conn,
}

impl Mysql {
    pub(crate) async fn connect(server: &Server) -> Result<Self, DbError> {
        let connection = server.connect_in_time(Self::open(server)).await?;

        Ok(Self { connection })
    }

    async fn open(server: &Server) -> Result<Conn, DbError> {
        match Conn::new(Opts::try_from(server)?).await {
            Err(error) if server.tls() == TlsMode::Prefer && lacks_tls(&error) => {
                let plain = server.clone().with_tls(TlsMode::Disable);
                Ok(Conn::new(Opts::try_from(&plain)?).await?)
            }
            connected => Ok(connected?),
        }
    }

    async fn version(&mut self) -> Result<String, DbError> {
        let version: Option<String> = self.connection.query_first(VERSION).await?;

        Ok(version.unwrap_or_default())
    }

    async fn run(&mut self, query: Query<'_>) -> Result<Rows, DbError> {
        if !query.access.is_read_only() {
            return self.execute(query).await;
        }

        self.read_only(async |mysql| mysql.execute(query).await)
            .await
    }

    async fn plan(&mut self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let explained = format!("EXPLAIN FORMAT=JSON {text}");
        let query = Query::for_one_value(&explained, params);

        let plan = self
            .read_only(async |mysql| mysql.execute(query).await)
            .await?
            .into_first_cell();

        let parsed = plan
            .as_str()
            .and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or(plan);

        Ok(parsed)
    }

    async fn table_names(&mut self) -> Result<Vec<TableName>, DbError> {
        self.read_only(async |mysql| mysql.catalog().tables().await)
            .await
    }

    async fn read_schema(&mut self) -> Result<Schema, DbError> {
        self.read_only(async |mysql| mysql.catalog().schema().await)
            .await
    }

    async fn read_only<T>(
        &mut self,
        work: impl AsyncFnOnce(&mut Self) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        for statement in [READ_ONLY_SESSION, READ_ONLY_TRANSACTION] {
            self.connection.query_drop(statement).await?;
        }

        let done = work(self).await;
        let _ended = self.connection.query_drop(ROLLBACK).await;

        done
    }

    async fn execute(&mut self, query: Query<'_>) -> Result<Rows, DbError> {
        if query.params.is_empty() && !query.access.is_read_only() {
            let result = self.connection.query_iter(query.text).await?;
            return collect(result, query).await;
        }

        let result = self
            .connection
            .exec_iter(query.text, params(query.params))
            .await?;

        collect(result, query).await
    }

    fn catalog(&mut self) -> Catalog<'_> {
        Catalog::new(&mut self.connection)
    }
}

impl Driver for Mysql {
    fn engine(&self) -> Engine {
        Engine::Mysql
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

    fn close(self: Box<Self>) -> futures_util::future::BoxFuture<'static, ()> {
        Box::pin(async move {
            let _closed = self.connection.disconnect().await;
        })
    }
}

fn lacks_tls(error: &Error) -> bool {
    matches!(error, Error::Driver(DriverError::NoClientSslFlagFromServer))
}
