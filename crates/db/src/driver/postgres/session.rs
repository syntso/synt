use futures_util::{TryStreamExt, pin_mut};
use serde_json::Value;
use tokio_postgres::{GenericClient, Statement};

use super::catalog::Catalog;
use super::cell::Cell;
use super::param::Param;
use crate::paging::Collector;
use crate::{DbError, Query, Rows};

const VERSION: &str = "SELECT current_setting('server_version') AS version";

pub(super) struct Session<'a, C> {
    client: &'a C,
}

impl<'a, C: GenericClient + Sync> Session<'a, C> {
    pub(super) fn new(client: &'a C) -> Self {
        Self { client }
    }

    pub(super) async fn version(&self) -> Result<String, DbError> {
        let row = self.client.query_one(VERSION, &[]).await?;

        row.try_get("version")
            .map_err(|_| DbError::unexpected_catalog_row())
    }

    pub(super) async fn run(&self, query: Query<'_>) -> Result<Rows, DbError> {
        let statement = self.prepare(query.text, query.params).await?;
        let params: Vec<Param> = query.params.iter().map(Param::from).collect();
        let columns = statement
            .columns()
            .iter()
            .map(|column| column.name().to_owned())
            .collect();

        let stream = self.client.query_raw(&statement, &params).await?;
        pin_mut!(stream);

        let mut collector = Collector::new(query.page, query.access);

        while let Some(row) = stream.try_next().await? {
            if collector.take(|| Cell::row(&row)).is_break() {
                break;
            }
        }

        Ok(collector.finish(columns, || stream.rows_affected()))
    }

    pub(super) async fn explain(&self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        let explained = format!("EXPLAIN (FORMAT JSON) {text}");
        let rows = self.run(Query::for_one_value(&explained, params)).await?;

        Ok(rows.into_first_cell())
    }

    pub(super) fn catalog(&self) -> Catalog<'a, C> {
        Catalog::new(self.client)
    }

    async fn prepare(&self, text: &str, params: &[Value]) -> Result<Statement, DbError> {
        let statement = self.client.prepare(text).await?;
        let expected = statement.params().len();

        if expected != params.len() {
            return Err(DbError::InvalidInput(format!(
                "the statement takes {expected} parameter(s), {} given",
                params.len()
            )));
        }

        Ok(statement)
    }
}
