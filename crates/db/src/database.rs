use serde_json::Value;

use crate::connection::Connection;
use crate::driver::{Mysql, Postgres, Redis, Sqlite};
use crate::{DbError, Engine, Page, Query, Rows, Schema, Server, TableName};

pub struct Database {
    engine: Engine,
    connection: Connection,
}

impl Database {
    pub async fn connect(server: &Server) -> Result<Self, DbError> {
        let connection = match server.engine() {
            Engine::Postgres => Connection::Postgres(Postgres::connect(server).await?),
            Engine::Mysql => Connection::Mysql(Mysql::connect(server).await?),
            Engine::Sqlite => Connection::Sqlite(Sqlite::new(server)?),
            Engine::Redis => Connection::Redis(Redis::connect(server).await?),
        };

        Ok(Self {
            engine: server.engine(),
            connection,
        })
    }

    pub fn engine(&self) -> Engine {
        self.engine
    }

    pub async fn ping(&mut self) -> Result<String, DbError> {
        match &mut self.connection {
            Connection::Postgres(postgres) => postgres.ping().await,
            Connection::Mysql(mysql) => mysql.ping().await,
            Connection::Sqlite(sqlite) => sqlite.ping().await,
            Connection::Redis(redis) => redis.ping().await,
        }
    }

    pub async fn query(&mut self, query: &Query<'_>) -> Result<Rows, DbError> {
        match &mut self.connection {
            Connection::Postgres(postgres) => postgres.query(query).await,
            Connection::Mysql(mysql) => mysql.query(query).await,
            Connection::Sqlite(sqlite) => sqlite.query(query).await,
            Connection::Redis(_) => Err(no_sql()),
        }
    }

    pub async fn command<S: AsRef<str>>(
        &mut self,
        words: &[S],
        page: Page,
    ) -> Result<Rows, DbError> {
        match &mut self.connection {
            Connection::Redis(redis) => redis.command(words, page).await,
            _ => Err(DbError::Unsupported(format!(
                "{} takes sql, not a command",
                self.engine
            ))),
        }
    }

    pub async fn explain(&mut self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        match &mut self.connection {
            Connection::Postgres(postgres) => postgres.explain(text, params).await,
            Connection::Mysql(mysql) => mysql.explain(text, params).await,
            Connection::Sqlite(sqlite) => sqlite.explain(text, params).await,
            Connection::Redis(_) => Err(no_sql()),
        }
    }

    pub async fn tables(&mut self) -> Result<Vec<TableName>, DbError> {
        match &mut self.connection {
            Connection::Postgres(postgres) => postgres.tables().await,
            Connection::Mysql(mysql) => mysql.tables().await,
            Connection::Sqlite(sqlite) => sqlite.tables().await,
            Connection::Redis(_) => Err(no_tables()),
        }
    }

    pub async fn schema(&mut self) -> Result<Schema, DbError> {
        match &mut self.connection {
            Connection::Postgres(postgres) => postgres.schema().await,
            Connection::Mysql(mysql) => mysql.schema().await,
            Connection::Sqlite(sqlite) => sqlite.schema().await,
            Connection::Redis(_) => Err(no_tables()),
        }
    }

    pub async fn close(self) {
        if let Connection::Mysql(mysql) = self.connection {
            mysql.close().await;
        }
    }
}

fn no_sql() -> DbError {
    DbError::Unsupported("redis takes commands, not sql".to_owned())
}

fn no_tables() -> DbError {
    DbError::Unsupported("redis has no tables; look at keys with SCAN".to_owned())
}
