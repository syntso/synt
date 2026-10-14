use serde_json::Value;

use crate::driver::Driver;
use crate::drivers::{Mysql, Postgres, Redis, Sqlite};
use crate::{DbError, Engine, Page, Query, Rows, Schema, Server, TableName};

pub struct Database {
    driver: Box<dyn Driver>,
}

impl Database {
    pub async fn connect(server: &Server) -> Result<Self, DbError> {
        let driver: Box<dyn Driver> = match server.engine() {
            Engine::Postgres => Box::new(Postgres::connect(server).await?),
            Engine::Mysql => Box::new(Mysql::connect(server).await?),
            Engine::Sqlite => Box::new(Sqlite::new(server)?),
            Engine::Redis => Box::new(Redis::connect(server).await?),
        };

        Ok(Self { driver })
    }

    pub fn engine(&self) -> Engine {
        self.driver.engine()
    }

    pub async fn ping(&mut self) -> Result<String, DbError> {
        self.driver.ping().await
    }

    pub async fn query(&mut self, query: &Query<'_>) -> Result<Rows, DbError> {
        self.driver.query(*query).await
    }

    pub async fn command<S: AsRef<str>>(
        &mut self,
        words: &[S],
        page: Page,
    ) -> Result<Rows, DbError> {
        let words: Vec<&str> = words.iter().map(AsRef::as_ref).collect();

        self.driver.command(&words, page).await
    }

    pub async fn explain(&mut self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        self.driver.explain(text, params).await
    }

    pub async fn tables(&mut self) -> Result<Vec<TableName>, DbError> {
        self.driver.tables().await
    }

    pub async fn schema(&mut self) -> Result<Schema, DbError> {
        self.driver.schema().await
    }

    pub async fn close(self) {
        self.driver.close().await;
    }
}
