mod mysql;
mod postgres;
mod redis;
mod sqlite;

use futures_util::future::{self, BoxFuture};
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
        refused(DbError::TakesCommands(self.engine()))
    }

    fn command<'a>(&'a mut self, _words: &'a [&'a str], _page: Page) -> Answer<'a, Rows> {
        refused(DbError::TakesSql(self.engine()))
    }

    fn explain<'a>(&'a mut self, _text: &'a str, _params: &'a [Value]) -> Answer<'a, Value> {
        refused(DbError::TakesCommands(self.engine()))
    }

    fn tables(&mut self) -> Answer<'_, Vec<TableName>> {
        refused(DbError::NoTables(self.engine()))
    }

    fn schema(&mut self) -> Answer<'_, Schema> {
        refused(DbError::NoTables(self.engine()))
    }

    fn close(self: Box<Self>) -> BoxFuture<'static, ()> {
        Box::pin(future::ready(()))
    }
}

fn refused<'a, T: Send + 'a>(error: DbError) -> Answer<'a, T> {
    Box::pin(future::ready(Err(error)))
}
