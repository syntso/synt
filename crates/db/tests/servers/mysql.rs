use serde_json::json;
use synt_db::{
    Access, Classification, Database, DbError, Effect, Engine, Page, Query, Server, TlsMode,
};
use synt_test_servers::DbServers;
use synt_testkit::TestError;

use crate::lab::{connect, read, run, server, write};

fn both(servers: &DbServers) -> [(&'static str, Server); 2] {
    [
        ("mysql", server(Engine::Mysql, servers.mysql_port())),
        ("mariadb", server(Engine::Mysql, servers.mariadb_port())),
    ]
}

fn refused_as_read_only(error: &DbError) -> bool {
    matches!(error, DbError::Server { code: Some(code), .. } if code.starts_with("1792"))
}

async fn refused(database: &mut Database, text: &str) -> Result<DbError, TestError> {
    let refusal = database
        .query(&Query {
            text,
            params: &[],
            access: Access::ReadOnly,
            page: Page::new(10, 1 << 20),
        })
        .await
        .err()
        .ok_or("the server ran a query it should refuse")?;

    Ok(refusal)
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn values_keep_their_types_in_both_protocols() -> Result<(), TestError> {
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        let mut database = connect(&server).await?;
        let text = "SELECT 42 AS answer, CAST(12.50 AS DECIMAL(6,2)) AS price, 'ann' AS name, \
                    CAST('2026-10-14 09:05:00' AS DATETIME) AS at, NULL AS nothing";

        let plain = read(&mut database, text).await;
        let prepared = run(
            &mut database,
            Access::ReadOnly,
            &format!("{text}, ? AS given"),
            &[json!(7)],
        )
        .await;

        let expected = vec![
            json!(42),
            json!("12.50"),
            json!("ann"),
            json!("2026-10-14 09:05:00"),
            json!(null),
        ];
        assert_eq!(plain.rows[0], expected, "{name}");
        assert_eq!(prepared.rows[0][..5], expected[..], "{name}");
        assert_eq!(prepared.rows[0][5], json!(7), "{name}");
    }

    Ok(())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_write_the_classifier_takes_for_a_read_is_refused_by_the_server() -> Result<(), TestError>
{
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        let mut database = connect(&server).await?;
        write(&mut database, "DROP TABLE IF EXISTS sneaked").await;
        write(&mut database, "CREATE TABLE sneaked (id int)").await;
        write(&mut database, "DROP FUNCTION IF EXISTS sneak").await;
        write(
            &mut database,
            "CREATE FUNCTION sneak() RETURNS int MODIFIES SQL DATA \
             BEGIN INSERT INTO sneaked VALUES (1); RETURN 1; END",
        )
        .await;
        let text = "SELECT sneak()";

        let classified = Classification::of_sql(Engine::Mysql, text);
        let error = refused(&mut database, text).await?;
        let left = read(&mut database, "SELECT count(*) FROM sneaked").await;

        assert_eq!(classified.effect, Effect::Read, "{name}");
        assert!(refused_as_read_only(&error), "{name}: {error:?}");
        assert_eq!(left.rows[0], vec![json!(0)], "{name}");
    }

    Ok(())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_read_carries_one_statement_even_when_the_server_lexes_backslashes_plainly()
-> Result<(), TestError> {
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        let mut database = connect(&server).await?;
        write(&mut database, "DROP TABLE IF EXISTS smuggled").await;
        write(&mut database, "CREATE TABLE smuggled (id int)").await;
        write(&mut database, "INSERT INTO smuggled VALUES (1)").await;
        write(
            &mut database,
            "SET SESSION sql_mode = CONCAT(@@sql_mode, ',NO_BACKSLASH_ESCAPES')",
        )
        .await;
        let text =
            r"SELECT 'a\'; COMMIT; SET SESSION TRANSACTION READ WRITE; DELETE FROM smuggled; -- '";

        let classified = Classification::of_sql(Engine::Mysql, text);
        let error = refused(&mut database, text).await?;
        let explained = database.explain(text, &[]).await;
        let left = read(&mut database, "SELECT count(*) FROM smuggled").await;

        assert_eq!(classified.effect, Effect::Read, "{name}");
        assert!(matches!(error, DbError::Server { .. }), "{name}: {error:?}");
        assert!(explained.is_err(), "{name}: {explained:?}");
        assert_eq!(left.rows[0], vec![json!(1)], "{name}");
    }

    Ok(())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn schema_changes_cannot_slip_out_of_a_read_by_committing_it() -> Result<(), TestError> {
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        let mut database = connect(&server).await?;
        write(&mut database, "DROP TABLE IF EXISTS slipped").await;

        let error = refused(&mut database, "CREATE TABLE slipped (id int)").await?;
        let tables = database.tables().await.unwrap();

        assert!(refused_as_read_only(&error), "{name}: {error:?}");
        assert!(
            !tables.iter().any(|table| table.name == "slipped"),
            "{name}"
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn the_schema_holds_keys_indexes_and_references() -> Result<(), TestError> {
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        let mut database = connect(&server).await?;
        write(&mut database, "DROP TABLE IF EXISTS shop_lines").await;
        write(&mut database, "DROP TABLE IF EXISTS shop_orders").await;
        write(
            &mut database,
            "CREATE TABLE shop_orders (region VARCHAR(8), id INT, note TEXT, \
             PRIMARY KEY (region, id))",
        )
        .await;
        write(
            &mut database,
            "CREATE TABLE shop_lines (region VARCHAR(8) NOT NULL, order_id INT NOT NULL, \
             sku VARCHAR(20), INDEX lines_by_sku (sku), \
             CONSTRAINT lines_order FOREIGN KEY (region, order_id) \
             REFERENCES shop_orders (region, id))",
        )
        .await;

        let schema = database.schema().await.unwrap();
        let orders = schema
            .tables
            .iter()
            .find(|table| table.name == "shop_orders")
            .unwrap();
        let lines = schema
            .tables
            .iter()
            .find(|table| table.name == "shop_lines")
            .unwrap();

        assert_eq!(
            orders.schema.as_deref(),
            Some(DbServers::DATABASE),
            "{name}"
        );
        assert_eq!(orders.primary_key, ["region", "id"], "{name}");
        assert_eq!(orders.columns[0].data_type, "varchar(8)", "{name}");
        assert_eq!(
            lines.foreign_keys[0].name.as_deref(),
            Some("lines_order"),
            "{name}"
        );
        assert_eq!(
            lines.foreign_keys[0].columns,
            ["region", "order_id"],
            "{name}"
        );
        assert_eq!(
            lines.foreign_keys[0].referenced_table, "shop_orders",
            "{name}"
        );
        assert!(
            lines
                .indexes
                .iter()
                .any(|index| index.name == "lines_by_sku"),
            "{name}"
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn explain_returns_the_plan_as_json() -> Result<(), TestError> {
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        let mut database = connect(&server).await?;

        let plan = database
            .explain(
                "SELECT * FROM information_schema.TABLES WHERE TABLE_NAME = ?",
                &[json!("x")],
            )
            .await
            .unwrap();

        assert!(plan["query_block"].is_object(), "{name}: {plan}");
    }

    Ok(())
}

#[tokio::test]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn tls_modes_encrypt_and_verify_full_refuses_an_untrusted_certificate()
-> Result<(), TestError> {
    let Some(servers) = DbServers::running() else {
        return Ok(());
    };

    for (name, server) in both(&servers) {
        for tls in [TlsMode::Disable, TlsMode::Prefer, TlsMode::Require] {
            let mut database = connect(&server.clone().with_tls(tls)).await?;
            let cipher = read(&mut database, "SHOW SESSION STATUS LIKE 'Ssl_cipher'").await;
            let encrypted = cipher.rows[0][1]
                .as_str()
                .is_some_and(|text| !text.is_empty());

            assert_eq!(encrypted, tls != TlsMode::Disable, "{name} {tls}");
        }

        let verified = Database::connect(&server.with_tls(TlsMode::VerifyFull)).await;

        assert!(verified.is_err(), "{name}");
    }

    Ok(())
}
