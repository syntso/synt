#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::{Grounds, Harness};
use rusqlite::Connection;
use serde_json::{Value, json};
use synt_client::Client;
use synt_core::{
    Broker, Core, Databases, ProjectOpen, Projects, RunbookActions, Runbooks, Secrets, Terminals,
};
use synt_journal::{Actor, Event, EventKind, Name};
use synt_proto::{ActionCall, ActionOutcome};
use synt_ssh::Pool;

const PROJECT_ID: &str = "5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d";

const SCHEMA: &str = "
    CREATE TABLE orders (id INTEGER PRIMARY KEY, state TEXT NOT NULL);
    INSERT INTO orders (state) VALUES ('new'), ('new'), ('paid');
";

const COUNT_NEW: &str = r#"
name = "Count new orders"

[[steps]]
title = "Count"
action = "db.query"
target = "@shop/orders"
params = { sql = "SELECT count(*) FROM orders WHERE state = 'new'" }
expect = { fields = { "/rows/0/0" = 2 } }
"#;

const CLOSE_NEW: &str = r#"
name = "Close new orders"

[[params]]
name = "state"
kind = "word"
default = "closed"

[[steps]]
title = "Close"
action = "db.query"
target = "@shop/orders"
params = { sql = "UPDATE orders SET state = '{state}' WHERE state = 'new'" }
expect = { fields = { "/affected" = 2 } }
"#;

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
        let file = folder.join("orders.sqlite");

        Connection::open(&file)
            .unwrap()
            .execute_batch(SCHEMA)
            .unwrap();
        write_file(&folder, "resources/orders.toml", &sqlite(&file));
        write_file(&folder, "runbooks/count-new.toml", COUNT_NEW);
        write_file(&folder, "runbooks/close-new.toml", CLOSE_NEW);

        let opening = ActionCall::new("project.open").params(json!({ "path": folder }));
        harness.app().await.call(opening).await.unwrap();

        Self { harness, file }
    }

    fn states(&self) -> Vec<String> {
        let connection = Connection::open(&self.file).unwrap();
        let mut statement = connection
            .prepare("SELECT state FROM orders ORDER BY id")
            .unwrap();

        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    async fn run(&self, client: &Client, name: &str, approve_steps: bool) -> ActionOutcome {
        let call = ActionCall::new("runbook.run").params(json!({
            "project": "shop",
            "name": name,
            "approve_steps": approve_steps,
        }));

        client.call(call).await.unwrap()
    }

    fn step_requests(&self) -> Vec<Event> {
        self.harness
            .events()
            .into_iter()
            .filter(|event| event.kind() == EventKind::Requested)
            .filter(|event| event.action().as_str() == "db.query")
            .collect()
    }
}

fn core(grounds: &Grounds) -> Arc<Core> {
    let projects = Projects::start(grounds.state()).unwrap();
    let secrets = Secrets::new(grounds.vault(), grounds.state());
    let terminals = Terminals::new(grounds.journal());
    let runbooks = Runbooks::new(projects.clone(), grounds.journal(), terminals);

    let mut registry = grounds.registry();
    registry
        .register(ProjectOpen::new(projects.clone()))
        .unwrap();
    Databases::new(Pool::new(), projects, secrets)
        .register(&mut registry)
        .unwrap();
    RunbookActions::new(runbooks.clone())
        .register(&mut registry)
        .unwrap();

    let core = Arc::new(Core::new(registry, grounds.journal(), Broker::new()));
    runbooks.connect(&core).unwrap();

    core
}

fn project(root: &Path) -> PathBuf {
    let folder = root.join("shop");
    write_file(
        &folder,
        "synt.toml",
        &format!("id = \"{PROJECT_ID}\"\nname = \"Shop\"\n"),
    );

    folder
}

fn write_file(folder: &Path, name: &str, text: &str) {
    let path = folder.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn sqlite(file: &Path) -> String {
    format!(
        "kind = \"database\"\nengine = \"sqlite\"\npath = {:?}\n",
        file.display().to_string()
    )
}

fn sql(event: &Event) -> Value {
    event.details().value()["params"]["sql"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_runs_a_reading_database_step_without_asking() {
    let shop = Shop::open().await;
    let mut approver = shop.harness.approver().await;
    let agent = shop.harness.agent("cursor").await;

    let outcome = shop.run(&agent, "count-new", false).await;
    let steps = shop.step_requests();

    assert_eq!(outcome.value["state"], json!("completed"));
    assert!(approver.was_not_asked());
    assert_eq!(steps.len(), 1);
    assert_eq!(
        steps[0].actor(),
        &Actor::Runbook(Name::new(format!("@{PROJECT_ID}/count-new")).unwrap())
    );
    assert_eq!(
        sql(&steps[0]),
        json!("SELECT count(*) FROM orders WHERE state = ?")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_writing_database_step_runs_inside_a_run_approved_up_front() {
    let shop = Shop::open().await;
    let app = shop.harness.app().await;

    let outcome = shop.run(&app, "close-new", true).await;
    let steps = shop.step_requests();

    assert_eq!(outcome.value["state"], json!("completed"));
    assert_eq!(shop.states(), ["closed", "closed", "paid"]);
    assert_eq!(
        sql(&steps[0]),
        json!("UPDATE orders SET state = ? WHERE state = ?")
    );
}
