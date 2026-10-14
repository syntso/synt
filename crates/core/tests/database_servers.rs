#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::Path;

use common::{Approver, Grounds, Harness, rejection};
use serde_json::{Value, json};
use synt_client::Client;
use synt_core::{Broker, Core, Databases, PasswordSet, Passwords, ProjectOpen, Projects};
use synt_journal::EventKind;
use synt_proto::{ActionCall, ApprovalDecision, ErrorCode, RiskLevel};
use synt_ssh::Pool;
use synt_test_servers::DbServers;

const PROJECT_FILE: &str = "id = \"8b8b8b8b8b8b8b8b8b8b8b8b8b8b8b8b\"\nname = \"Data\"\n";
const POSTGRES: &str = "@data/postgres";
const MYSQL: &str = "@data/mysql";
const REDIS: &str = "@data/redis";

struct Data {
    harness: Harness,
    app: Client,
    agent: Client,
}

impl Data {
    async fn start(servers: &DbServers) -> Self {
        let grounds = Grounds::new();
        let core = core(&grounds);
        let harness = Harness::serve(grounds, core);
        let app = harness.app().await;
        let agent = harness.agent("cursor").await;

        store_password(&app, "test-db", DbServers::PASSWORD).await;
        let folder = project(harness.grounds().path(), servers);
        let opening = ActionCall::new("project.open").params(json!({ "path": folder }));
        app.call(opening).await.unwrap();

        Self {
            harness,
            app,
            agent,
        }
    }

    async fn query(&self, target: &str, params: Value) -> Result<Value, synt_client::ClientError> {
        let call = ActionCall::new("db.query").target(target).params(params);

        self.agent.call(call).await.map(|outcome| outcome.value)
    }

    fn journal_text(&self) -> String {
        format!("{:?}", self.harness.events())
    }
}

fn core(grounds: &Grounds) -> Core {
    let projects = Projects::start(grounds.state()).unwrap();
    let passwords = Passwords::new(grounds.vault(), grounds.state());
    let mut registry = grounds.registry();

    registry
        .register(ProjectOpen::new(projects.clone()))
        .unwrap();
    registry
        .register(PasswordSet::new(passwords.clone()))
        .unwrap();
    Databases::new(Pool::new(), projects, passwords)
        .register(&mut registry)
        .unwrap();

    Core::new(registry, grounds.journal(), Broker::new())
}

async fn store_password(app: &Client, name: &str, password: &str) {
    let call =
        ActionCall::new("password.set").params(json!({ "name": name, "password": password }));

    app.call(call).await.unwrap();
}

fn project(root: &Path, servers: &DbServers) -> std::path::PathBuf {
    let folder = root.join("data");
    fs::create_dir_all(folder.join("resources")).unwrap();
    fs::write(folder.join("synt.toml"), PROJECT_FILE).unwrap();

    let resources = [
        (
            "postgres",
            "postgres",
            servers.postgres_port(),
            "require",
            true,
        ),
        ("mysql", "mysql", servers.mysql_port(), "prefer", true),
        ("redis", "redis", servers.redis_port(), "disable", false),
    ];

    for (name, engine, port, tls, named) in resources {
        let identity = if named {
            format!(
                "database = \"{}\"\nuser = \"{}\"\n",
                DbServers::DATABASE,
                DbServers::USER
            )
        } else {
            String::new()
        };
        let text = format!(
            "kind = \"database\"\nengine = \"{engine}\"\nhost = \"{}\"\nport = {port}\n\
             {identity}password = \"secret://test-db\"\ntls = \"{tls}\"\n",
            DbServers::ADDRESS
        );
        fs::write(folder.join(format!("resources/{name}.toml")), text).unwrap();
    }

    folder
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_read_runs_with_the_vault_password_and_answers_over_tls() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let data = Data::start(&servers).await;

    let rows = data
        .query(
            POSTGRES,
            json!({ "sql": "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()" }),
        )
        .await
        .unwrap();

    assert_eq!(rows["rows"], json!([[true]]));
    assert!(!data.journal_text().contains(DbServers::PASSWORD));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_hidden_write_in_a_read_is_stopped_by_the_server_and_journaled_as_failed() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let data = Data::start(&servers).await;
    let mut approver = Approver::new(data.app.clone());
    let setup = [
        "DROP TABLE IF EXISTS core_sneaked",
        "CREATE TABLE core_sneaked (id int)",
        "CREATE OR REPLACE FUNCTION core_sneak() RETURNS int LANGUAGE sql AS \
         'INSERT INTO core_sneaked VALUES (1) RETURNING id'",
    ];
    for sql in setup {
        let call = ActionCall::new("db.query")
            .target(POSTGRES)
            .params(json!({ "sql": sql }));
        approver
            .approve(&data.agent, call, ApprovalDecision::AllowOnce)
            .await;
    }

    let refused = rejection(
        data.query(POSTGRES, json!({ "sql": "SELECT core_sneak()" }))
            .await,
    );
    let left = data
        .query(
            POSTGRES,
            json!({ "sql": "SELECT count(*) FROM core_sneaked" }),
        )
        .await
        .unwrap();
    let failed = data
        .harness
        .events()
        .into_iter()
        .rev()
        .find(|event| event.kind() == EventKind::Failed)
        .unwrap();

    assert!(approver.was_not_asked());
    assert_eq!(refused.code, ErrorCode::ACTION_FAILED);
    assert!(refused.message.contains("25006"), "{}", refused.message);
    assert_eq!(left["rows"], json!([[0]]));
    assert_eq!(failed.action().as_str(), "db.query");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn redis_writes_are_asked_and_journaled_without_their_values() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let data = Data::start(&servers).await;
    let mut approver = Approver::new(data.app.clone());
    let set = ActionCall::new("db.query")
        .target(REDIS)
        .params(json!({ "command": ["SET", "synt:core:token", "token-value-42"] }));

    let (question, outcome) = approver
        .decide(&data.agent, set, ApprovalDecision::AllowOnce)
        .await;
    let read = data
        .query(REDIS, json!({ "command": ["GET", "synt:core:token"] }))
        .await
        .unwrap();

    assert_eq!(question.risk, RiskLevel::Write);
    assert_eq!(outcome.unwrap().value["rows"], json!([["OK"]]));
    assert_eq!(read["rows"], json!([["token-value-42"]]));
    assert!(!data.journal_text().contains("token-value-42"));
    assert!(!data.journal_text().contains("synt:core:token"));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn the_schema_of_a_mysql_database_comes_through_the_core() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let data = Data::start(&servers).await;
    let mut approver = Approver::new(data.app.clone());
    for sql in [
        "DROP TABLE IF EXISTS core_listed",
        "CREATE TABLE core_listed (id INT PRIMARY KEY, label VARCHAR(10))",
    ] {
        let call = ActionCall::new("db.query")
            .target(MYSQL)
            .params(json!({ "sql": sql }));
        approver
            .approve(&data.agent, call, ApprovalDecision::AllowOnce)
            .await;
    }

    let schema = data
        .agent
        .call(ActionCall::new("db.schema").target(MYSQL))
        .await
        .unwrap()
        .value;
    let listed = schema["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["name"] == json!("core_listed"))
        .unwrap();

    assert_eq!(listed["primary_key"], json!(["id"]));
    assert_eq!(listed["columns"][1]["data_type"], json!("varchar(10)"));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the database test servers: tests/db/up.sh, then SYNT_DB_TESTS=1"]
async fn a_wrong_password_is_reported_without_being_shown() {
    let Some(servers) = DbServers::running() else {
        return;
    };
    let data = Data::start(&servers).await;
    store_password(&data.app, "test-db", "not-the-password").await;

    let refused = rejection(data.query(POSTGRES, json!({ "sql": "SELECT 1" })).await);

    assert_eq!(refused.code, ErrorCode::ACTION_FAILED);
    assert!(refused.message.contains("28P01"), "{}", refused.message);
    assert!(!refused.message.contains("not-the-password"));
    assert!(!data.journal_text().contains("not-the-password"));
}
