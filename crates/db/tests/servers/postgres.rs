use serde_json::json;
use synt_db::{
    Access, Classification, Database, DbError, Effect, Engine, Page, Query, Server, TlsMode,
};
use synt_test_servers::DbServers;
use zeroize::Zeroizing;

use crate::servers::{connect, read, run, server, with_tls, write};

fn postgres(servers: &DbServers) -> Server {
    server(Engine::Postgres, servers.postgres_port())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn values_keep_their_types_on_the_way_out() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = connect(&postgres(&servers)).await;

    let rows = read(
        &mut database,
        "SELECT 42::int4 AS answer, 12.50::numeric AS price, 'ann'::text AS name, \
                '{\"a\": [1, 2]}'::jsonb AS doc, true AS flag, NULL::int AS nothing, \
                '2026-10-14 09:05:00.25+00'::timestamptz AS at, '\\x00ff'::bytea AS raw, \
                ARRAY[1, 2, 3] AS list, '6f1c0a52-4d5e-4f8a-9b1c-2d3e4f5a6b7c'::uuid AS id",
    )
    .await;

    assert_eq!(
        rows.columns,
        [
            "answer", "price", "name", "doc", "flag", "nothing", "at", "raw", "list", "id"
        ]
    );
    assert_eq!(
        rows.rows[0],
        vec![
            json!(42),
            json!("12.50"),
            json!("ann"),
            json!({"a": [1, 2]}),
            json!(true),
            json!(null),
            json!("2026-10-14 09:05:00.25+00"),
            json!("\\x00ff"),
            json!([1, 2, 3]),
            json!("6f1c0a52-4d5e-4f8a-9b1c-2d3e4f5a6b7c"),
        ]
    );
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn json_parameters_reach_typed_columns() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = connect(&postgres(&servers)).await;
    let params = [json!(7), json!("seven"), json!(null)];

    let rows = run(
        &mut database,
        Access::ReadOnly,
        "SELECT $1::int8 + 1, upper($2), $3::date IS NULL",
        &params,
    )
    .await;

    assert_eq!(rows.rows[0], vec![json!(8), json!("SEVEN"), json!(true)]);
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_write_the_classifier_takes_for_a_read_is_refused_by_the_server() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = connect(&postgres(&servers)).await;
    write(&mut database, "DROP TABLE IF EXISTS sneaked").await;
    write(&mut database, "CREATE TABLE sneaked (id int)").await;
    write(
        &mut database,
        "CREATE OR REPLACE FUNCTION sneak() RETURNS int LANGUAGE sql AS \
         'INSERT INTO sneaked VALUES (1) RETURNING id'",
    )
    .await;
    let text = "SELECT sneak()";

    let classified = Classification::of_sql(Engine::Postgres, text);
    let refused = database
        .query(&Query {
            text,
            params: &[],
            access: Access::ReadOnly,
            page: Page::new(10, 1 << 20),
        })
        .await
        .unwrap_err();
    let left = read(&mut database, "SELECT count(*) FROM sneaked").await;

    assert_eq!(classified.effect, Effect::Read);
    assert!(
        matches!(&refused, DbError::Server { code: Some(code), .. } if code == "25006"),
        "{refused:?}"
    );
    assert_eq!(left.rows[0], vec![json!(0)]);
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_write_counts_every_row_it_changed_while_returning_one_page() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = connect(&postgres(&servers)).await;
    write(&mut database, "DROP TABLE IF EXISTS counted").await;
    write(&mut database, "CREATE TABLE counted (id int)").await;

    let rows = database
        .query(&Query {
            text: "INSERT INTO counted SELECT generate_series(1, 25) RETURNING id",
            params: &[],
            access: Access::ReadWrite,
            page: Page::new(10, 1 << 20),
        })
        .await
        .unwrap();
    let stored = read(&mut database, "SELECT count(*) FROM counted").await;

    assert_eq!(rows.count(), 10);
    assert!(rows.more);
    assert_eq!(rows.affected, Some(25));
    assert_eq!(stored.rows[0], vec![json!(25)]);
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn the_schema_holds_keys_indexes_and_references() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = connect(&postgres(&servers)).await;
    write(&mut database, "DROP SCHEMA IF EXISTS shop CASCADE").await;
    write(&mut database, "CREATE SCHEMA shop").await;
    write(
        &mut database,
        "CREATE TABLE shop.orders (region text, id int, placed date DEFAULT now(), \
         PRIMARY KEY (region, id))",
    )
    .await;
    write(
        &mut database,
        "CREATE TABLE shop.lines (region text NOT NULL, order_id int NOT NULL, sku text, \
         CONSTRAINT lines_order FOREIGN KEY (region, order_id) REFERENCES shop.orders (region, id))",
    )
    .await;
    write(
        &mut database,
        "CREATE INDEX lines_by_sku ON shop.lines (sku)",
    )
    .await;

    let schema = database.schema().await.unwrap();
    let orders = schema
        .tables
        .iter()
        .find(|table| table.schema.as_deref() == Some("shop") && table.name == "orders")
        .unwrap();
    let lines = schema
        .tables
        .iter()
        .find(|table| table.schema.as_deref() == Some("shop") && table.name == "lines")
        .unwrap();

    assert_eq!(orders.primary_key, ["region", "id"]);
    assert_eq!(orders.columns[2].data_type, "date");
    assert_eq!(orders.columns[2].default.as_deref(), Some("now()"));
    assert!(!lines.columns[0].nullable);
    assert_eq!(lines.foreign_keys[0].name.as_deref(), Some("lines_order"));
    assert_eq!(lines.foreign_keys[0].columns, ["region", "order_id"]);
    assert_eq!(lines.foreign_keys[0].referenced_columns, ["region", "id"]);
    assert!(
        lines
            .indexes
            .iter()
            .any(|index| index.name == "lines_by_sku")
    );

    let tables = database.tables().await.unwrap();
    assert!(
        tables
            .iter()
            .any(|table| table.name == "lines" && table.kind == "table")
    );
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn explain_returns_the_plan_as_json_and_changes_nothing() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let mut database = connect(&postgres(&servers)).await;
    write(&mut database, "DROP TABLE IF EXISTS planned").await;
    write(&mut database, "CREATE TABLE planned (id int)").await;
    write(&mut database, "INSERT INTO planned VALUES (1)").await;

    let plan = database
        .explain("DELETE FROM planned WHERE id = $1", &[json!(1)])
        .await
        .unwrap();
    let left = read(&mut database, "SELECT count(*) FROM planned").await;

    assert!(plan[0]["Plan"]["Node Type"].is_string(), "{plan}");
    assert_eq!(left.rows[0], vec![json!(1)]);
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn tls_modes_encrypt_and_verify_full_refuses_an_untrusted_certificate() {
    let Some(servers) = DbServers::running() else {
        return;
    };

    for tls in [TlsMode::Disable, TlsMode::Prefer, TlsMode::Require] {
        let mut database = connect(&with_tls(postgres(&servers), tls)).await;
        let ssl = read(
            &mut database,
            "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
        )
        .await;

        assert_eq!(ssl.rows[0], vec![json!(tls != TlsMode::Disable)], "{tls}");
    }

    let refused = Database::connect(&with_tls(postgres(&servers), TlsMode::VerifyFull)).await;

    assert!(refused.is_err());
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_server_reached_through_a_local_port_is_still_named_by_its_host() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let address = format!("127.0.0.1:{}", servers.postgres_port());
    let through = Server::network(Engine::Postgres, "orders-db.internal", 5432)
        .reached_at(address.parse().unwrap())
        .with_database(Some(DbServers::DATABASE.to_owned()))
        .with_user(Some(DbServers::USER.to_owned()))
        .with_password(Some(Zeroizing::new(DbServers::PASSWORD.to_owned())))
        .with_tls(TlsMode::Require);

    let mut database = connect(&through).await;

    assert!(!database.ping().await.unwrap().is_empty());
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_wrong_password_is_the_server_saying_no() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let wrong = postgres(&servers).with_password(Some(Zeroizing::new("wrong".to_owned())));

    let refused = Database::connect(&wrong).await.err().unwrap();

    assert!(
        matches!(&refused, DbError::Server { code: Some(code), .. } if code == "28P01"),
        "{refused:?}"
    );
    assert!(!format!("{refused:?}").contains(DbServers::PASSWORD));
}
