mod catalog;
mod cell;
mod error;
mod numeric;
mod options;
mod session;
mod text_param;

use std::sync::Arc;

use rustls::ClientConfig;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Config, Connection, NoTls, Transaction};
use tokio_postgres_rustls::MakeRustlsConnect;

use self::session::Session;
use crate::driver::{Answer, Driver};
use crate::{DbError, Engine, Query, Rows, Schema, Server, TableName};

pub(crate) struct Postgres {
    client: Client,
    background: JoinHandle<()>,
}

impl Postgres {
    pub(crate) async fn connect(server: &Server) -> Result<Self, DbError> {
        let config = Config::try_from(server)?;
        let tls = server.tls().client_config()?;

        server.connect_in_time(Self::open(config, tls)).await
    }

    async fn open(config: Config, tls: Option<Arc<ClientConfig>>) -> Result<Self, DbError> {
        let Some(tls) = tls else {
            let (client, connection) = config.connect(NoTls).await?;
            return Ok(Self::driven(client, connection));
        };

        let maker = MakeRustlsConnect::new((*tls).clone());
        let (client, connection) = config.connect(maker).await?;

        Ok(Self::driven(client, connection))
    }

    fn driven<S, T>(client: Client, connection: Connection<S, T>) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let background = tokio::spawn(async move {
            let _ended = connection.await;
        });

        Self { client, background }
    }

    async fn version(&self) -> Result<String, DbError> {
        Session::new(&self.client).version().await
    }

    async fn run(&mut self, query: Query<'_>) -> Result<Rows, DbError> {
        if !query.access.is_read_only() {
            return Session::new(&self.client).run(query).await;
        }

        self.read_only(async |session| session.run(query).await)
            .await
    }

    async fn plan(&mut self, text: &str, params: &[Value]) -> Result<Value, DbError> {
        self.read_only(async |session| session.explain(text, params).await)
            .await
    }

    async fn table_names(&mut self) -> Result<Vec<TableName>, DbError> {
        self.read_only(async |session| session.catalog().tables().await)
            .await
    }

    async fn read_schema(&mut self) -> Result<Schema, DbError> {
        self.read_only(async |session| session.catalog().schema().await)
            .await
    }

    async fn read_only<T>(
        &mut self,
        work: impl AsyncFnOnce(&Session<'_, Transaction<'_>>) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        let transaction = self
            .client
            .build_transaction()
            .read_only(true)
            .start()
            .await?;

        let done = work(&Session::new(&transaction)).await;
        let _ended = transaction.rollback().await;

        done
    }
}

impl Driver for Postgres {
    fn engine(&self) -> Engine {
        Engine::Postgres
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

impl Drop for Postgres {
    fn drop(&mut self) {
        self.background.abort();
    }
}
