mod mysql;
mod postgres;
mod redis;
mod sqlite;

use std::future;

use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::{DbError, Engine, Page, Query, Rows, Schema, TableName};

pub(crate) use mysql::Mysql;
pub(crate) use postgres::Postgres;
pub(crate) use redis::Redis;
pub(crate) use sqlite::Sqlite;

pub(crate) type Answer<'a, T> = BoxFuture<'a, Result<T, DbError>>;

pub(crate) trait Driver: Send {
    fn engine(&self) -> Engine;

    fn ping(&mut self) -> Answer<'_, String>;

    fn query<'a>(&'a mut self, _query: Query<'a>) -> Answer<'a, Rows> {
        unsupported(format!("{} takes commands, not sql", self.engine()))
    }

    fn command<'a>(&'a mut self, _words: &'a [&'a str], _page: Page) -> Answer<'a, Rows> {
        unsupported(format!("{} takes sql, not a command", self.engine()))
    }

    fn explain<'a>(&'a mut self, _text: &'a str, _params: &'a [Value]) -> Answer<'a, Value> {
        unsupported(format!("{} takes commands, not sql", self.engine()))
    }

    fn tables(&mut self) -> Answer<'_, Vec<TableName>> {
        unsupported(no_tables(self.engine()))
    }

    fn schema(&mut self) -> Answer<'_, Schema> {
        unsupported(no_tables(self.engine()))
    }

    fn close(self: Box<Self>) -> BoxFuture<'static, ()> {
        Box::pin(future::ready(()))
    }
}

fn no_tables(engine: Engine) -> String {
    format!("{engine} has no tables; look at keys with SCAN")
}

fn unsupported<'a, T: Send + 'a>(reason: String) -> Answer<'a, T> {
    Box::pin(future::ready(Err(DbError::Unsupported(reason))))
}
