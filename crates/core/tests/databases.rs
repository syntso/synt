#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Approver, Grounds, Harness, rejection};
use rusqlite::Connection;
use serde_json::{Value, json};
use synt_client::Client;
use synt_core::{
    Broker, Core, Databases, ProjectOpen, Projects, SecretList, SecretPut, SecretRemove, Secrets,
};
use synt_journal::EventKind;
use synt_proto::{ActionCall, ActionOutcome, ApprovalDecision, ErrorCode, RiskLevel};
use synt_ssh::Pool;

const PROJECT_FILE: &str = "id = \"6d6d6d6d6d6d6d6d6d6d6d6d6d6d6d6d\"\nname = \"Shop\"\n";
const SHOP: &str = "@shop/shop";

const SCHEMA: &str = "
    CREATE TABLE customers (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE, secret TEXT);
    CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER NOT NULL REFERENCES customers(id),
        total REAL
    );
    INSERT INTO customers VALUES (1, 'ann@example.com', 'ann-private-note');
    INSERT INTO customers VALUES (2, 'bob@example.com', 'bob-private-note');
    INSERT INTO orders (customer_id, total) VALUES (1, 10), (1, 20), (2, 30);
";

struct Shop {
    harness: Harness,
    file: PathBuf,
}

impl Shop {
    async fn open() -> Self {
        let grounds = Grounds::new();
        let core = core(&grounds);
        let harness = Harness::serve(grounds, core);
        let folder = project(harness.grounds().path());
        let file = folder.join("shop.sqlite");

        Connection::open(&file)
            .unwrap()
            .execute_batch(SCHEMA)
            .unwrap();
        resource(&folder, "shop", &sqlite_resource(&file));
        harness
            .app()
            .await
            .call(ActionCall::new("project.open").params(json!({ "path": folder })))
            .await
            .unwrap();

        Self { harness, file }
    }

    fn count(&self, table: &str) -> i64 {
        Connection::open(&self.file)
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn journal_text(&self) -> String {
        format!("{:?}", self.harness.events())
    }

    fn details(&self, outcome: &ActionOutcome, kind: EventKind) -> Value {
        self.harness
            .trail(outcome.request)
            .into_iter()
            .find(|event| event.kind() == kind)
            .unwrap_or_else(|| panic!("no {kind:?} for {:?}", outcome.request))
            .details()
            .value()
            .clone()
    }
}

fn core(grounds: &Grounds) -> Core {
    let projects = Projects::start(grounds.state()).unwrap();
    let secrets = Secrets::new(grounds.vault(), grounds.state());
    let mut registry = grounds.registry();

    registry
        .register(ProjectOpen::new(projects.clone()))
        .unwrap();
    registry.register(SecretPut::new(secrets.clone())).unwrap();
    registry.register(SecretList::new(secrets.clone())).unwrap();
    registry
        .register(SecretRemove::new(secrets.clone()))
        .unwrap();
    Databases::new(Pool::new(), projects, secrets)
        .register(&mut registry)
        .unwrap();

    Core::new(registry, grounds.journal(), Broker::new())
}

fn project(root: &Path) -> PathBuf {
    let folder = root.join("shop");
    fs::create_dir_all(folder.join("resources")).unwrap();
    fs::write(folder.join("synt.toml"), PROJECT_FILE).unwrap();

    folder
}

fn resource(folder: &Path, name: &str, text: &str) {
    fs::write(folder.join(format!("resources/{name}.toml")), text).unwrap();
}

fn sqlite_resource(file: &Path) -> String {
    format!(
        "kind = \"database\"\nengine = \"sqlite\"\npath = {:?}\n",
        file.display().to_string()
    )
}

fn query(sql: &str) -> ActionCall {
    ActionCall::new("db.query")
        .target(SHOP)
        .params(json!({ "sql": sql }))
}

async fn agent(shop: &Shop) -> Client {
    shop.harness.agent("cursor").await
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_reads_without_asking_and_gets_typed_rows() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;

    let outcome = agent
        .call(query("SELECT id, email FROM customers ORDER BY id"))
        .await
        .unwrap();

    assert_eq!(outcome.value["columns"], json!(["id", "email"]));
    assert_eq!(
        outcome.value["rows"],
        json!([[1, "ann@example.com"], [2, "bob@example.com"]])
    );
    assert_eq!(outcome.value["effect"], json!("read"));
    assert_eq!(outcome.value["kind"], json!("select"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_journal_keeps_the_masked_statement_and_counts_but_never_the_data() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let call = ActionCall::new("db.query").target(SHOP).params(json!({
        "sql": "SELECT secret FROM customers WHERE email = 'ann@example.com' OR id = ?1",
        "params": [2],
    }));

    let outcome = agent.call(call).await.unwrap();
    let requested = shop.details(&outcome, EventKind::Requested);
    let finished = shop.details(&outcome, EventKind::Finished);
    let journal = shop.journal_text();

    assert_eq!(outcome.value["rows"].as_array().unwrap().len(), 2);
    assert_eq!(
        requested["params"]["sql"],
        json!("SELECT secret FROM customers WHERE email = ? OR id = ?1")
    );
    assert_eq!(requested["params"]["params"], json!(1));
    assert_eq!(finished["rows"], json!(2));
    assert!(finished["duration_ms"].is_u64());
    for private in ["ann@example.com", "ann-private-note", "bob-private-note"] {
        assert!(!journal.contains(private), "the journal holds {private}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_write_from_an_agent_waits_for_the_person_and_a_denial_changes_nothing() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let mut approver = Approver::new(shop.harness.app().await);

    let (question, outcome) = approver
        .decide(
            &agent,
            query("DELETE FROM orders WHERE total > 15"),
            ApprovalDecision::Deny,
        )
        .await;

    assert_eq!(question.risk, RiskLevel::Write);
    assert_eq!(
        question.params["sql"],
        json!("DELETE FROM orders WHERE total > 15")
    );
    assert_eq!(question.params["judged"]["kind"], json!("delete"));
    assert_eq!(rejection(outcome).code, ErrorCode::DENIED);
    assert_eq!(shop.count("orders"), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_approved_write_runs_and_reports_the_rows_it_changed() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let mut approver = Approver::new(shop.harness.app().await);

    let outcome = approver
        .approve(
            &agent,
            query("UPDATE orders SET total = total * 2 WHERE customer_id = 1"),
            ApprovalDecision::AllowOnce,
        )
        .await;
    let finished = shop.details(&outcome, EventKind::Finished);

    assert_eq!(outcome.value["affected"], json!(2));
    assert_eq!(finished["affected"], json!(2));
    assert_eq!(finished["effect"], json!("write"));
}

#[tokio::test(flavor = "multi_thread")]
async fn schema_changes_and_unreadable_text_are_asked_as_privileged() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let mut approver = Approver::new(shop.harness.app().await);

    for sql in [
        "DROP TABLE orders",
        "SELECT 1; DELETE FROM orders",
        "SELEKT everything",
    ] {
        let (question, outcome) = approver
            .decide(&agent, query(sql), ApprovalDecision::Deny)
            .await;

        assert_eq!(question.risk, RiskLevel::Privileged, "{sql}");
        assert_eq!(rejection(outcome).code, ErrorCode::DENIED, "{sql}");
    }

    assert_eq!(shop.count("orders"), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn pages_are_fetched_by_offset_until_nothing_is_left() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let page = |offset: u64| {
        ActionCall::new("db.query").target(SHOP).params(json!({
            "sql": "SELECT id FROM orders ORDER BY id",
            "limit": 2,
            "offset": offset,
        }))
    };

    let first = agent.call(page(0)).await.unwrap().value;
    let next = first["next_offset"].as_u64().unwrap();
    let second = agent.call(page(next)).await.unwrap().value;

    assert_eq!(first["rows"], json!([[1], [2]]));
    assert_eq!(first["more"], json!(true));
    assert_eq!(second["rows"], json!([[3]]));
    assert_eq!(second["more"], json!(false));
    assert_eq!(second["next_offset"], json!(null));
}

#[tokio::test(flavor = "multi_thread")]
async fn schema_tables_explain_and_ping_answer_for_a_database_resource() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let call = |action: &str, params: Value| ActionCall::new(action).target(SHOP).params(params);

    let schema = agent
        .call(call("db.schema", json!(null)))
        .await
        .unwrap()
        .value;
    let tables = agent
        .call(call("db.tables", json!(null)))
        .await
        .unwrap()
        .value;
    let plan = agent
        .call(call(
            "db.explain",
            json!({ "sql": "SELECT * FROM orders WHERE customer_id = 1" }),
        ))
        .await
        .unwrap()
        .value;
    let ping = agent
        .call(call("db.ping", json!(null)))
        .await
        .unwrap()
        .value;

    let orders = &schema["tables"][1];
    assert_eq!(orders["name"], json!("orders"));
    assert_eq!(orders["primary_key"], json!(["id"]));
    assert_eq!(
        orders["foreign_keys"][0]["referenced_table"],
        json!("customers")
    );
    assert_eq!(tables["tables"].as_array().unwrap().len(), 2);
    assert!(
        plan["plan"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
    assert_eq!(ping["engine"], json!("sqlite"));
}

#[tokio::test(flavor = "multi_thread")]
async fn explain_takes_exactly_one_statement() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let call = ActionCall::new("db.explain")
        .target(SHOP)
        .params(json!({ "sql": "SELECT 1; DROP TABLE orders" }));

    let error = rejection(agent.call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert_eq!(shop.count("orders"), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_redis_command_is_refused_for_an_sql_database() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;
    let call = ActionCall::new("db.query")
        .target(SHOP)
        .params(json!({ "command": ["GET", "x"] }));

    let error = rejection(agent.call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(error.message.contains("takes sql"), "{}", error.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_target_that_is_not_a_database_resource_fails_clearly() {
    let shop = Shop::open().await;
    let app = shop.harness.app().await;
    let call = ActionCall::new("db.ping").target("@shop/missing");

    let error = rejection(app.call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS, "{}", error.message);
    assert!(
        error.message.contains("no resource missing"),
        "{}",
        error.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn database_actions_are_described_with_their_risks_and_params() {
    let shop = Shop::open().await;
    let agent = agent(&shop).await;

    let described = agent.describe().await.unwrap();
    let find = |id: &str| described.iter().find(|action| action.id == id).unwrap();

    assert_eq!(find("db.query").risk, RiskLevel::Write);
    assert!(find("db.query").params.to_string().contains("\"sql\""));
    assert_eq!(find("db.schema").risk, RiskLevel::Read);
    assert_eq!(find("db.tables").risk, RiskLevel::Read);
    assert_eq!(find("db.explain").risk, RiskLevel::Read);
    assert_eq!(find("db.ping").risk, RiskLevel::Read);
    assert!(find("db.ping").takes_target);
    assert!(described.iter().all(|action| action.id != "secret.put"));
}
