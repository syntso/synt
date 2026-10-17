#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{Value, json};
use synt_db::{Access, Database, DbError, Page, Query, Server};
use tempfile::TempDir;

const SHOP: &str = "
    CREATE TABLE customers (
        id INTEGER PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        note TEXT DEFAULT 'none'
    );
    CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER NOT NULL REFERENCES customers(id),
        total REAL,
        receipt BLOB
    );
    CREATE INDEX orders_by_customer ON orders (customer_id, total);
    CREATE VIEW big_orders AS SELECT * FROM orders WHERE total > 100;
    INSERT INTO customers (id, email) VALUES (1, 'ann@example.com'), (2, 'bob@example.com');
    INSERT INTO orders (customer_id, total, receipt) VALUES (1, 250.5, x'00ff'), (2, 12, NULL), (1, 99, NULL);
";

struct Shop {
    _folder: TempDir,
    path: PathBuf,
}

impl Shop {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("shop.sqlite");

        Connection::open(&path)
            .unwrap()
            .execute_batch(SHOP)
            .unwrap();

        Self {
            _folder: folder,
            path,
        }
    }

    async fn open(&self) -> Database {
        Database::connect(&Server::file(&self.path)).await.unwrap()
    }

    fn count(&self, table: &str) -> i64 {
        Connection::open(&self.path)
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}

fn read(text: &str) -> Query<'_> {
    Query {
        text,
        params: &[],
        access: Access::ReadOnly,
        page: Page::new(100, 1 << 20),
    }
}

fn write(text: &str) -> Query<'_> {
    Query {
        access: Access::ReadWrite,
        ..read(text)
    }
}

#[tokio::test]
async fn a_read_returns_columns_and_typed_rows() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let rows = database
        .query(&read("SELECT id, total, receipt FROM orders ORDER BY id"))
        .await
        .unwrap();

    assert_eq!(rows.columns, ["id", "total", "receipt"]);
    assert_eq!(rows.rows[0], vec![json!(1), json!(250.5), json!("\\x00ff")]);
    assert_eq!(rows.rows[1], vec![json!(2), json!(12.0), Value::Null]);
    assert!(!rows.more);
    assert_eq!(rows.affected, None);
}

#[tokio::test]
async fn parameters_are_bound_not_spliced() {
    let shop = Shop::new();
    let mut database = shop.open().await;
    let params = [json!("ann@example.com' OR '1'='1")];

    let rows = database
        .query(&Query {
            params: &params,
            ..read("SELECT id FROM customers WHERE email = ?1")
        })
        .await
        .unwrap();

    assert!(rows.rows.is_empty());
}

#[tokio::test]
async fn pages_follow_each_other_without_gaps() {
    let shop = Shop::new();
    let mut database = shop.open().await;
    let text = "SELECT id FROM orders ORDER BY id";

    let first = database
        .query(&Query {
            page: Page::new(2, 1 << 20),
            ..read(text)
        })
        .await
        .unwrap();
    let second = database
        .query(&Query {
            page: Page::new(2, 1 << 20).starting_at(first.next_offset.unwrap()),
            ..read(text)
        })
        .await
        .unwrap();

    assert_eq!(first.rows, vec![vec![json!(1)], vec![json!(2)]]);
    assert!(first.more);
    assert_eq!(second.rows, vec![vec![json!(3)]]);
    assert!(!second.more);
}

#[tokio::test]
async fn a_write_reports_how_many_rows_it_changed() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let rows = database
        .query(&write(
            "UPDATE orders SET total = total + 1 WHERE customer_id = 1",
        ))
        .await
        .unwrap();

    assert_eq!(rows.affected, Some(2));
}

#[tokio::test]
async fn a_write_sent_down_the_read_path_is_refused_by_sqlite() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let refused = database
        .query(&read("DELETE FROM orders"))
        .await
        .unwrap_err();

    assert!(matches!(refused, DbError::Server { .. }), "{refused:?}");
    assert_eq!(shop.count("orders"), 3);
}

#[tokio::test]
async fn several_statements_are_refused_rather_than_partly_run() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let refused = database
        .query(&write("DELETE FROM orders; DELETE FROM customers"))
        .await
        .unwrap_err();

    assert!(matches!(refused, DbError::InvalidInput(_)), "{refused:?}");
    assert_eq!(shop.count("customers"), 2);
}

#[tokio::test]
async fn a_missing_file_is_not_created() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("absent.sqlite");
    let mut database = Database::connect(&Server::file(&path)).await.unwrap();

    let failed = database.query(&write("CREATE TABLE t (id int)")).await;

    assert!(
        matches!(failed, Err(DbError::Unopened { .. })),
        "{failed:?}"
    );
    assert!(!Path::new(&path).exists());
}

#[tokio::test]
async fn the_schema_names_tables_columns_keys_indexes_and_references() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let schema = database.schema().await.unwrap();
    let names: Vec<(&str, &str)> = schema
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table.kind.name()))
        .collect();
    let orders = schema
        .tables
        .iter()
        .find(|table| table.name == "orders")
        .unwrap();
    let customers = schema
        .tables
        .iter()
        .find(|table| table.name == "customers")
        .unwrap();

    assert_eq!(
        names,
        [
            ("big_orders", "view"),
            ("customers", "table"),
            ("orders", "table")
        ]
    );
    assert_eq!(orders.primary_key, ["id"]);
    assert_eq!(orders.columns[1].name, "customer_id");
    assert_eq!(orders.columns[1].data_type, "INTEGER");
    assert!(!orders.columns[1].nullable);
    assert_eq!(customers.columns[2].default.as_deref(), Some("'none'"));
    assert_eq!(orders.indexes[0].name, "orders_by_customer");
    assert_eq!(orders.indexes[0].columns, ["customer_id", "total"]);
    assert!(
        customers
            .indexes
            .iter()
            .any(|index| index.unique && index.columns == ["email"])
    );
    assert_eq!(orders.foreign_keys[0].columns, ["customer_id"]);
    assert_eq!(orders.foreign_keys[0].referenced_table, "customers");
    assert_eq!(orders.foreign_keys[0].referenced_columns, ["id"]);
}

#[tokio::test]
async fn tables_lists_names_only() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let tables = database.tables().await.unwrap();

    assert_eq!(tables.len(), 3);
    assert_eq!(tables[0].name, "big_orders");
}

#[tokio::test]
async fn explain_shows_the_query_plan_without_running_it() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let plan = database
        .explain("DELETE FROM orders WHERE customer_id = ?1", &[json!(1)])
        .await
        .unwrap();

    assert!(
        plan.as_array().is_some_and(|steps| !steps.is_empty()),
        "{plan}"
    );
    assert_eq!(shop.count("orders"), 3);
}

#[tokio::test]
async fn ping_answers_with_the_library_version() {
    let shop = Shop::new();
    let mut database = shop.open().await;

    let version = database.ping().await.unwrap();

    assert!(version.starts_with('3'), "{version}");
}
