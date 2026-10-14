#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use common::{Grounds, Harness, rejection};
use serde_json::json;
use synt_client::Client;
use synt_core::{
    Broker, Core, Databases, PasswordList, PasswordRemove, PasswordSet, Passwords, ProjectOpen,
    Projects, ResourceList,
};
use synt_journal::EventKind;
use synt_proto::{ActionCall, ErrorCode};
use synt_ssh::Pool;

const PROJECT_FILE: &str = "id = \"7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a7a\"\nname = \"Vaulted\"\n";
const PASSWORD: &str = "correct horse battery staple";

fn start() -> Harness {
    let grounds = Grounds::new();
    let projects = Projects::start(grounds.state()).unwrap();
    let passwords = Passwords::new(grounds.vault(), grounds.state());
    let mut registry = grounds.registry();

    registry
        .register(ProjectOpen::new(projects.clone()))
        .unwrap();
    registry
        .register(ResourceList::new(projects.clone()))
        .unwrap();
    registry
        .register(PasswordSet::new(passwords.clone()))
        .unwrap();
    registry
        .register(PasswordList::new(passwords.clone()))
        .unwrap();
    registry
        .register(PasswordRemove::new(passwords.clone()))
        .unwrap();
    Databases::new(Pool::new(), projects, passwords)
        .register(&mut registry)
        .unwrap();

    let core = Core::new(registry, grounds.journal(), Broker::new());

    Harness::serve(grounds, core)
}

fn set(name: &str) -> ActionCall {
    ActionCall::new("password.set").params(json!({ "name": name, "password": PASSWORD }))
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
async fn a_password_goes_into_the_vault_and_never_into_the_journal() {
    let harness = start();
    let app = harness.app().await;

    let stored = app.call(set("prod/orders-db")).await.unwrap();
    let listed = app.call(ActionCall::new("password.list")).await.unwrap();
    let requested = harness.trail(stored.request)[0].details().value().clone();

    assert_eq!(stored.value["reference"], json!("secret://prod/orders-db"));
    assert_eq!(listed.value[0]["name"], json!("prod/orders-db"));
    assert_eq!(requested["params"], json!({ "name": "prod/orders-db" }));
    assert!(
        harness
            .grounds()
            .vault_items()
            .contains(&"password.prod.orders-db".to_owned())
    );
    assert!(!format!("{:?}", harness.events()).contains(PASSWORD));
    assert!(!listed.value.to_string().contains(PASSWORD));
}

#[tokio::test(flavor = "multi_thread")]
async fn setting_a_password_again_replaces_it() {
    let harness = start();
    let app = harness.app().await;

    app.call(set("orders-db")).await.unwrap();
    app.call(set("orders-db")).await.unwrap();
    let listed = app.call(ActionCall::new("password.list")).await.unwrap();

    assert_eq!(listed.value.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_person_sets_or_removes_passwords() {
    let harness = start();
    let agent = harness.agent("cursor").await;

    let setting = rejection(agent.call(set("orders-db")).await);
    let removing = rejection(
        agent
            .call(ActionCall::new("password.remove").params(json!({ "name": "orders-db" })))
            .await,
    );

    assert_eq!(setting.code, ErrorCode::DENIED);
    assert_eq!(removing.code, ErrorCode::DENIED);
    assert!(
        harness
            .grounds()
            .vault_items()
            .iter()
            .all(|item| !item.starts_with("password."))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_removed_password_leaves_the_vault() {
    let harness = start();
    let app = harness.app().await;
    app.call(set("orders-db")).await.unwrap();

    let removed = app
        .call(ActionCall::new("password.remove").params(json!({ "name": "orders-db" })))
        .await
        .unwrap();
    let again = rejection(
        app.call(ActionCall::new("password.remove").params(json!({ "name": "orders-db" })))
            .await,
    );

    assert_eq!(removed.value["name"], json!("orders-db"));
    assert!(
        !harness
            .grounds()
            .vault_items()
            .contains(&"password.orders-db".to_owned())
    );
    assert!(
        again.message.contains("no password named orders-db"),
        "{}",
        again.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_database_naming_a_missing_password_fails_before_connecting() {
    let harness = start();
    let app = harness.app().await;
    let orders = format!(
        "kind = \"database\"\nengine = \"postgres\"\nhost = \"127.0.0.1\"\nport = {}\n\
         user = \"app\"\npassword = \"secret://orders-db\"\n",
        closed_port()
    );
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
    assert!(error.message.contains("password.set"), "{}", error.message);
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
