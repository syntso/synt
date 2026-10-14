#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use common::{Grounds, Harness, rejection};
use serde_json::json;
use synt_client::Client;
use synt_core::{
    Broker, Core, Databases, ProjectOpen, Projects, ResourceList, ResourcePut, SecretList,
    SecretPut, SecretRemove, Secrets,
};
use synt_journal::EventKind;
use synt_proto::{ActionCall, ErrorCode};
use synt_ssh::Pool;

const PROJECT_FILE: &str = "id = \"7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a\"\nname = \"Vaulted\"\n";
const PASSWORD: &str = "correct horse battery staple";

fn start() -> Harness {
    let grounds = Grounds::new();
    let projects = Projects::start(grounds.state()).unwrap();
    let secrets = Secrets::new(grounds.vault(), grounds.state());
    let mut registry = grounds.registry();

    registry
        .register(ProjectOpen::new(projects.clone()))
        .unwrap();
    registry
        .register(ResourceList::new(projects.clone()))
        .unwrap();
    registry
        .register(ResourcePut::new(projects.clone()))
        .unwrap();
    registry.register(SecretPut::new(secrets.clone())).unwrap();
    registry.register(SecretList::new(secrets.clone())).unwrap();
    registry
        .register(SecretRemove::new(secrets.clone()))
        .unwrap();
    Databases::new(Pool::new(), projects, secrets)
        .register(&mut registry)
        .unwrap();

    let core = Core::new(registry, grounds.journal(), Broker::new());

    Harness::serve(grounds, core)
}

fn put(name: &str) -> ActionCall {
    ActionCall::new("secret.put").params(json!({ "name": name, "value": PASSWORD }))
}

fn remove(name: &str) -> ActionCall {
    ActionCall::new("secret.remove").params(json!({ "name": name }))
}

fn database(password: &str) -> String {
    format!(
        "kind = \"database\"\nengine = \"postgres\"\nhost = \"127.0.0.1\"\nport = {}\n\
         user = \"app\"\npassword = \"{password}\"\n",
        closed_port()
    )
}

fn project(root: &Path, resources: &[(&str, String)]) -> PathBuf {
    let folder = root.join("vaulted");
    fs::create_dir_all(folder.join("resources")).unwrap();
    fs::write(folder.join("synt.toml"), PROJECT_FILE).unwrap();

    for (name, text) in resources {
        fs::write(folder.join(format!("resources/{name}.toml")), text).unwrap();
    }

    folder
}

async fn open(app: &Client, folder: &Path) {
    let call = ActionCall::new("project.open").params(json!({ "path": folder }));

    app.call(call).await.unwrap();
}

fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();

    listener.local_addr().unwrap().port()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_secret_goes_into_the_vault_and_never_into_the_journal() {
    let harness = start();
    let app = harness.app().await;

    let stored = app.call(put("prod/orders-db")).await.unwrap();
    let listed = app.call(ActionCall::new("secret.list")).await.unwrap();
    let requested = harness.trail(stored.request)[0].details().value().clone();

    assert_eq!(stored.value["reference"], json!("secret://prod/orders-db"));
    assert_eq!(listed.value[0]["name"], json!("prod/orders-db"));
    assert_eq!(
        requested["params"],
        json!({ "name": "prod/orders-db", "value_bytes": PASSWORD.len() })
    );
    assert!(
        harness
            .grounds()
            .vault_items()
            .contains(&"secret.prod.orders-db".to_owned())
    );
    assert!(!format!("{:?}", harness.events()).contains(PASSWORD));
    assert!(!listed.value.to_string().contains(PASSWORD));
}

#[tokio::test(flavor = "multi_thread")]
async fn putting_a_secret_again_replaces_it() {
    let harness = start();
    let app = harness.app().await;

    app.call(put("orders-db")).await.unwrap();
    app.call(put("orders-db")).await.unwrap();
    let listed = app.call(ActionCall::new("secret.list")).await.unwrap();

    assert_eq!(listed.value.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_person_puts_or_removes_secrets() {
    let harness = start();
    let agent = harness.agent("cursor").await;

    let putting = rejection(agent.call(put("orders-db")).await);
    let removing = rejection(agent.call(remove("orders-db")).await);

    assert_eq!(putting.code, ErrorCode::DENIED);
    assert_eq!(removing.code, ErrorCode::DENIED);
    assert!(
        harness
            .grounds()
            .vault_items()
            .iter()
            .all(|item| !item.starts_with("secret."))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_removed_secret_leaves_the_vault() {
    let harness = start();
    let app = harness.app().await;
    app.call(put("orders-db")).await.unwrap();

    let removed = app.call(remove("orders-db")).await.unwrap();
    let again = rejection(app.call(remove("orders-db")).await);

    assert_eq!(removed.value["name"], json!("orders-db"));
    assert!(
        !harness
            .grounds()
            .vault_items()
            .contains(&"secret.orders-db".to_owned())
    );
    assert!(
        again.message.contains("no secret orders-db"),
        "{}",
        again.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_database_naming_a_missing_secret_fails_before_connecting() {
    let harness = start();
    let app = harness.app().await;
    let orders = database("secret://orders-db");
    open(
        &app,
        &project(harness.grounds().path(), &[("orders", orders)]),
    )
    .await;

    let error = rejection(
        app.call(ActionCall::new("db.ping").target("@vaulted/orders"))
            .await,
    );
    let failed = harness
        .events()
        .into_iter()
        .find(|event| event.kind() == EventKind::Failed)
        .unwrap();

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(
        error.message.contains("secret://orders-db"),
        "{}",
        error.message
    );
    assert!(error.message.contains("secret.put"), "{}", error.message);
    assert!(
        failed
            .details()
            .value()
            .to_string()
            .contains("secret://orders-db")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_password_written_into_a_resource_file_is_a_problem_not_a_resource() {
    let harness = start();
    let app = harness.app().await;
    let leaked = "kind = \"database\"\nengine = \"mysql\"\nhost = \"db\"\npassword = \"hunter2\"\n";
    open(
        &app,
        &project(harness.grounds().path(), &[("leaked", leaked.to_owned())]),
    )
    .await;

    let listed = app
        .call(ActionCall::new("resource.list").params(json!({ "project": "vaulted" })))
        .await
        .unwrap()
        .value;

    assert_eq!(listed["resources"], json!([]));
    assert!(
        listed["problems"][0]["error"]
            .as_str()
            .unwrap()
            .contains("secret://")
    );
    assert!(!listed.to_string().contains("hunter2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_password_sent_with_a_resource_never_reaches_the_journal() {
    let harness = start();
    let app = harness.app().await;
    open(&app, &project(harness.grounds().path(), &[])).await;

    let call = ActionCall::new("resource.put")
        .target("@vaulted/orders-db")
        .params(json!({
            "kind": "database",
            "engine": "mysql",
            "host": "db",
            "password": "hunter2",
        }));
    let refused = rejection(app.call(call).await);

    assert_eq!(refused.code, ErrorCode::ACTION_FAILED);
    assert!(!format!("{:?}", harness.events()).contains("hunter2"));
    assert!(format!("{:?}", harness.events()).contains("orders-db"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_database_reads_its_password_from_a_kept_secret() {
    let harness = start();
    let app = harness.app().await;
    app.call(put("orders-db")).await.unwrap();
    let orders = database("secret://orders-db");
    open(
        &app,
        &project(harness.grounds().path(), &[("orders", orders)]),
    )
    .await;

    let error = rejection(
        app.call(ActionCall::new("db.ping").target("@vaulted/orders"))
            .await,
    );

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(!error.message.contains("secret.put"), "{}", error.message);
    assert!(!error.message.contains(PASSWORD), "{}", error.message);
    assert!(!format!("{:?}", harness.events()).contains(PASSWORD));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kept_password_goes_only_to_the_database_it_was_first_used_for() {
    let harness = start();
    let app = harness.app().await;
    app.call(put("orders-db")).await.unwrap();
    let orders = database("secret://orders-db");
    let elsewhere = database("secret://orders-db").replace("127.0.0.1", "localhost");
    open(
        &app,
        &project(
            harness.grounds().path(),
            &[("orders", orders), ("elsewhere", elsewhere)],
        ),
    )
    .await;
    let ping = |name: &str| ActionCall::new("db.ping").target(format!("@vaulted/{name}"));

    let first = rejection(app.call(ping("orders")).await);
    let refused = rejection(app.call(ping("elsewhere")).await);
    let again = rejection(app.call(ping("orders")).await);

    assert!(!first.message.contains("kept for"), "{}", first.message);
    assert!(
        refused
            .message
            .contains("secret://orders-db is kept for the database postgres app@127.0.0.1"),
        "{}",
        refused.message
    );
    assert!(
        refused.message.contains("secret.put"),
        "{}",
        refused.message
    );
    assert!(!again.message.contains("kept for"), "{}", again.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn putting_a_password_again_lets_it_go_to_a_new_database() {
    let harness = start();
    let app = harness.app().await;
    app.call(put("orders-db")).await.unwrap();
    let orders = database("secret://orders-db");
    let elsewhere = database("secret://orders-db").replace("127.0.0.1", "localhost");
    open(
        &app,
        &project(
            harness.grounds().path(),
            &[("orders", orders), ("elsewhere", elsewhere)],
        ),
    )
    .await;
    let ping = |name: &str| ActionCall::new("db.ping").target(format!("@vaulted/{name}"));

    rejection(app.call(ping("orders")).await);
    app.call(put("orders-db")).await.unwrap();
    let moved = rejection(app.call(ping("elsewhere")).await);
    let old = rejection(app.call(ping("orders")).await);

    assert!(!moved.message.contains("kept for"), "{}", moved.message);
    assert!(
        old.message
            .contains("kept for the database postgres app@localhost"),
        "{}",
        old.message
    );
}
