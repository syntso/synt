use serde_json::Value;
use synt_db::{Access, Database, Engine, Page, Query, Rows, Server};
use synt_test_servers::DbServers;
use zeroize::Zeroizing;

pub fn server(engine: Engine, port: u16) -> Server {
    Server::network(engine, DbServers::ADDRESS, port)
        .with_database(Some(DbServers::DATABASE.to_owned()))
        .with_user(Some(DbServers::USER.to_owned()))
        .with_password(Some(Zeroizing::new(DbServers::PASSWORD.to_owned())))
}

pub async fn connect(server: &Server) -> Database {
    Database::connect(server).await.unwrap()
}

pub async fn run(database: &mut Database, access: Access, text: &str, params: &[Value]) -> Rows {
    database
        .query(&Query {
            text,
            params,
            access,
            page: Page::new(100, 1 << 20),
        })
        .await
        .unwrap_or_else(|error| panic!("{text}: {error}"))
}

pub async fn write(database: &mut Database, text: &str) -> Rows {
    run(database, Access::ReadWrite, text, &[]).await
}

pub async fn read(database: &mut Database, text: &str) -> Rows {
    run(database, Access::ReadOnly, text, &[]).await
}
