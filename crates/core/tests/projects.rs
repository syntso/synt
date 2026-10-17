use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use synt_actions::Action;
use synt_client::Client;
use synt_core::{
    Broker, Core, ProjectClose, ProjectList, ProjectOpen, Projects, ResourceGet, ResourceList,
    ResourcePut, ResourceRemove, SshExec,
};
use synt_proto::{ActionCall, ApprovalDecision, ErrorCode, ResourceRef, Risk};
use synt_ssh::Pool;
use synt_testkit::TestError;
use synt_testkit::core::{Grounds, Harness, eventually, rejection};

const PROJECT_ID: &str = "4f2c9a7e1b3d5f60718293a4b5c6d7e8";

const PROJECT_FILE: &str = r#"
id = "4f2c9a7e1b3d5f60718293a4b5c6d7e8"
name = "Orders Platform"
"#;

const BASTION: &str = r#"
kind = "host"
address = "10.0.0.1"
port = 2222
user = "deploy"
"#;

const API: &str = r#"
kind = "host"
address = "10.0.1.12"
user = "deploy"
via = "bastion"
"#;

const MOVED_API: &str = r#"
kind = "host"
address = "10.0.9.9"
user = "deploy"
via = "bastion"
"#;

fn core(grounds: &Grounds) -> Result<Core, TestError> {
    let projects = Projects::start(grounds.state()?)?;
    let mut registry = grounds.registry()?;

    registry.register(ProjectOpen::new(projects.clone()))?;
    registry.register(ProjectClose::new(projects.clone()))?;
    registry.register(ProjectList::new(projects.clone()))?;
    registry.register(ResourceList::new(projects.clone()))?;
    registry.register(ResourceGet::new(projects.clone()))?;
    registry.register(ResourcePut::new(projects.clone()))?;
    registry.register(ResourceRemove::new(projects.clone()))?;
    registry.register(SshExec::new(Pool::new(), projects))?;

    Ok(Core::new(registry, grounds.journal(), Broker::new()))
}

fn start() -> Result<Harness, TestError> {
    let grounds = Grounds::new()?;
    let core = core(&grounds)?;

    Harness::serve(grounds, core)
}

fn orders(harness: &Harness) -> Result<PathBuf, TestError> {
    let folder = harness.grounds().path().join("orders");
    let resources = folder.join("resources");
    fs::create_dir_all(&resources)?;

    fs::write(folder.join("synt.toml"), PROJECT_FILE)?;
    fs::write(resources.join("bastion.toml"), BASTION)?;
    fs::write(resources.join("api-1.toml"), API)?;

    Ok(folder)
}

fn resource_file(folder: &Path, name: &str) -> PathBuf {
    folder.join("resources").join(format!("{name}.toml"))
}

async fn open(app: &Client, folder: &Path) -> Result<Value, TestError> {
    let call = ActionCall::new(ProjectOpen::ID).params(json!({ "path": folder }));

    Ok(app.call(call).await?.value)
}

async fn listing(app: &Client) -> Result<Value, TestError> {
    let call = ActionCall::new(ResourceList::ID).params(json!({ "project": "orders" }));

    Ok(app.call(call).await?.value)
}

async fn address(app: &Client, name: &str) -> Option<String> {
    let call =
        ActionCall::new(ResourceGet::ID).params(json!({ "project": "orders", "name": name }));
    let entry = app.call(call).await.ok()?.value;

    entry["address"].as_str().map(str::to_owned)
}

async fn names(app: &Client) -> Result<Vec<String>, TestError> {
    let listed = listing(app).await?;

    let resources = listed["resources"]
        .as_array()
        .ok_or("the listing has no resources")?;

    Ok(resources
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .map(str::to_owned)
        .collect())
}

async fn problems(app: &Client) -> Result<Vec<Value>, TestError> {
    let listed = listing(app).await?;
    let problems = listed["problems"]
        .as_array()
        .ok_or("the listing has no problems list")?;

    Ok(problems.clone())
}

fn exec(target: &str) -> ActionCall {
    ActionCall::new(SshExec::ID)
        .target(target)
        .params(json!({ "command": "uptime" }))
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_a_plain_folder_gives_it_an_id_and_a_resources_folder() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = harness.grounds().path().join("billing");
    fs::create_dir(&folder).unwrap();

    let info = open(&app, &folder).await?;
    let written = fs::read_to_string(folder.join("synt.toml")).unwrap();
    let id = info["id"].as_str().unwrap();

    assert_eq!(id.len(), 32);
    assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
    assert!(written.contains(id));
    assert_eq!(info["name"], json!("billing"));
    assert_eq!(info["slug"], json!("billing"));
    assert_eq!(info["read_only"], json!(false));
    assert_eq!(info["resources"], json!(0));
    assert!(folder.join("resources").is_dir());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_reads_the_project_file_and_the_resources_already_there() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;

    let info = open(&app, &folder).await?;

    assert_eq!(info["id"], json!(PROJECT_ID));
    assert_eq!(info["name"], json!("Orders Platform"));
    assert_eq!(info["resources"], json!(2));
    assert_eq!(names(&app).await?, ["api-1", "bastion"]);
    assert_eq!(address(&app, "api-1").await.as_deref(), Some("10.0.1.12"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_the_same_folder_twice_keeps_one_project() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;

    let first = open(&app, &folder).await?;
    let second = open(&app, &folder).await?;
    let listed = app
        .call(ActionCall::new(ProjectList::ID))
        .await
        .unwrap()
        .value;

    assert_eq!(first, second);
    assert_eq!(
        listed
            .as_array()
            .unwrap()
            .iter()
            .filter(|project| project["slug"] == json!("orders"))
            .count(),
        1
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_folder_with_the_same_id_is_refused() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    let copy = harness.grounds().path().join("orders-copy");
    fs::create_dir(&copy).unwrap();
    fs::write(copy.join("synt.toml"), PROJECT_FILE).unwrap();

    open(&app, &folder).await?;
    let call = ActionCall::new(ProjectOpen::ID).params(json!({ "path": copy }));
    let error = rejection(app.call(call).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(error.message.contains("same id"), "{}", error.message);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relative_path_is_refused() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;

    let call = ActionCall::new(ProjectOpen::ID).params(json!({ "path": "orders" }));
    let error = rejection(app.call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_and_closing_projects_is_for_the_person_only() -> Result<(), TestError> {
    let harness = start()?;
    let agent = harness.agent("deploy-bot").await?;
    let folder = orders(&harness)?;

    let opening = ActionCall::new(ProjectOpen::ID).params(json!({ "path": folder }));
    let closing = ActionCall::new(ProjectClose::ID).params(json!({ "id": "orders" }));

    assert_eq!(rejection(agent.call(opening).await).code, ErrorCode::DENIED);
    assert_eq!(rejection(agent.call(closing).await).code, ErrorCode::DENIED);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_resource_file_added_later_is_loaded() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    fs::write(resource_file(&folder, "api-2"), API).unwrap();

    eventually("api-2 to be loaded", async || {
        names(&app)
            .await
            .ok()?
            .contains(&"api-2".to_owned())
            .then_some(())
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_resource_file_is_reloaded() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    fs::write(resource_file(&folder, "api-1"), MOVED_API).unwrap();

    eventually("api-1 to be reloaded", async || {
        (address(&app, "api-1").await.as_deref() == Some("10.0.9.9")).then_some(())
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_resource_file_is_forgotten() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    fs::remove_file(resource_file(&folder, "api-1")).unwrap();

    eventually("api-1 to be forgotten", async || {
        (names(&app).await.ok()? == ["bastion"]).then_some(())
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_file_is_reported_and_the_last_good_version_stays_in_use() -> Result<(), TestError>
{
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    fs::write(
        resource_file(&folder, "api-1"),
        "kind = \"host\"\naddress = ",
    )
    .unwrap();
    let reported = eventually("the broken file to be reported", async || {
        problems(&app).await.ok()?.pop()
    })
    .await;

    assert_eq!(reported["file"], json!("resources/api-1.toml"));
    assert!(
        reported["error"]
            .as_str()
            .unwrap()
            .ends_with("the last good version stays in use"),
        "{reported}"
    );
    assert_eq!(address(&app, "api-1").await.as_deref(), Some("10.0.1.12"));

    fs::write(resource_file(&folder, "api-1"), MOVED_API).unwrap();
    eventually("the fixed file to be loaded", async || {
        let fixed = problems(&app).await.ok()?.is_empty()
            && address(&app, "api-1").await.as_deref() == Some("10.0.9.9");
        fixed.then_some(())
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_file_that_is_broken_is_reported_and_not_loaded() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    fs::write(resource_file(&folder, "db"), "kind = \"database\"").unwrap();
    eventually("the broken file to be reported", async || {
        problems(&app).await.ok()?.pop()
    })
    .await;

    let call =
        ActionCall::new(ResourceGet::ID).params(json!({ "project": "orders", "name": "db" }));
    let error = rejection(app.call(call).await);

    assert!(
        error.message.contains("its file is broken"),
        "{}",
        error.message
    );
    assert_eq!(names(&app).await?, ["api-1", "bastion"]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_target_resolves_through_its_via_chain() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    open(approver.app(), &orders(&harness)?).await?;

    let (question, _) = approver
        .decide(&agent, exec("@orders/api-1"), ApprovalDecision::Deny)
        .await?;

    assert_eq!(
        question.target.as_ref().map(ResourceRef::as_str),
        Some("@orders/api-1")
    );
    assert_eq!(
        question.destination.as_deref(),
        Some(
            format!("@{PROJECT_ID}/api-1 = deploy@10.0.1.12:22 via [deploy@10.0.0.1:2222]")
                .as_str()
        )
    );
    assert_eq!(question.risk, Risk::Write);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_can_be_named_by_its_id() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    open(approver.app(), &orders(&harness)?).await?;

    let (question, _) = approver
        .decide(
            &agent,
            exec(&format!("@{PROJECT_ID}/bastion")),
            ApprovalDecision::Deny,
        )
        .await?;

    assert_eq!(
        question.destination.as_deref(),
        Some(format!("@{PROJECT_ID}/bastion = deploy@10.0.0.1:2222").as_str())
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_host_that_logs_in_as_root_is_asked_as_privileged() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let folder = orders(&harness)?;
    fs::write(
        resource_file(&folder, "db-1"),
        "kind = \"host\"\naddress = \"10.0.2.5\"\nuser = \"root\"\n",
    )
    .unwrap();
    open(approver.app(), &folder).await?;

    let (question, _) = approver
        .decide(&agent, exec("@orders/db-1"), ApprovalDecision::Deny)
        .await?;

    assert_eq!(question.risk, Risk::Privileged);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_via_chain_that_loops_names_no_host() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let folder = orders(&harness)?;
    fs::write(
        resource_file(&folder, "bastion"),
        "kind = \"host\"\naddress = \"10.0.0.1\"\nuser = \"deploy\"\nvia = \"api-1\"\n",
    )
    .unwrap();
    open(approver.app(), &folder).await?;

    let error = rejection(agent.call(exec("@orders/api-1")).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(approver.was_not_asked());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_approved_for_one_place_does_not_run_after_its_file_moves_it()
-> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let folder = orders(&harness)?;
    open(approver.app(), &folder).await?;

    let running = tokio::spawn(async move { agent.call(exec("@orders/api-1")).await });
    let question = approver.next().await?;
    fs::write(resource_file(&folder, "api-1"), MOVED_API).unwrap();
    eventually("api-1 to be reloaded", async || {
        (address(approver.app(), "api-1").await.as_deref() == Some("10.0.9.9")).then_some(())
    })
    .await;
    approver
        .answer(&question, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let error = rejection(running.await.unwrap());

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(error.message.contains("now leads to"), "{}", error.message);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_writes_resources_as_files() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    let call = ActionCall::new(ResourcePut::ID)
        .target("@orders/web-1")
        .params(json!({ "kind": "host", "address": "10.0.3.3", "user": "deploy" }));
    app.call(call).await.unwrap();
    let written = fs::read_to_string(resource_file(&folder, "web-1")).unwrap();

    let removing = ActionCall::new(ResourceRemove::ID).target("@orders/api-1");
    app.call(removing).await.unwrap();

    assert!(written.contains("10.0.3.3"), "{written}");
    assert_eq!(address(&app, "web-1").await.as_deref(), Some("10.0.3.3"));
    assert!(!resource_file(&folder, "api-1").exists());
    assert_eq!(names(&app).await?, ["bastion", "web-1"]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_changing_a_resource_is_asked_with_the_project_id() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    open(approver.app(), &orders(&harness)?).await?;

    let call = ActionCall::new(ResourcePut::ID)
        .target("@orders/web-1")
        .params(json!({ "kind": "host", "address": "10.0.3.3" }));
    let (question, outcome) = approver
        .decide(&agent, call, ApprovalDecision::Deny)
        .await?;

    assert_eq!(question.risk, Risk::Write);
    assert_eq!(
        question.destination.as_deref(),
        Some(format!("@{PROJECT_ID}/web-1").as_str())
    );
    assert_eq!(rejection(outcome).code, ErrorCode::DENIED);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_resource_that_fails_its_checks_is_not_written() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    let call = ActionCall::new(ResourcePut::ID)
        .target("@orders/web-1")
        .params(json!({ "kind": "host", "address": "10.0.3.3", "port": 0 }));
    let error = rejection(app.call(call).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(!resource_file(&folder, "web-1").exists());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_resource_is_written_as_a_file_and_listed() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    let call = ActionCall::new(ResourcePut::ID)
        .target("@orders/kube")
        .params(json!({
            "kind": "cluster",
            "kubeconfig": "~/.kube/orders.yaml",
            "context": "prod-eu",
            "namespace": "orders",
            "via": "bastion",
        }));
    app.call(call).await.unwrap();
    let written = fs::read_to_string(resource_file(&folder, "kube")).unwrap();
    let listed = listing(&app).await?;
    let kube = listed["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "kube")
        .unwrap();

    assert!(written.starts_with("kind = \"cluster\""), "{written}");
    assert!(written.contains("via = \"bastion\""), "{written}");
    assert_eq!(kube["context"], "prod-eu");
    assert_eq!(kube["kubeconfig"], "~/.kube/orders.yaml");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_is_neither_an_ssh_target_nor_a_way_in() -> Result<(), TestError> {
    let harness = start()?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let folder = orders(&harness)?;
    fs::write(
        resource_file(&folder, "kube"),
        "kind = \"cluster\"\ncontext = \"prod-eu\"\nvia = \"bastion\"\n",
    )
    .unwrap();
    fs::write(
        resource_file(&folder, "behind"),
        "kind = \"host\"\naddress = \"10.0.4.4\"\nuser = \"deploy\"\nvia = \"kube\"\n",
    )
    .unwrap();
    open(approver.app(), &folder).await?;

    let on_cluster = rejection(agent.call(exec("@orders/kube")).await);
    let through_cluster = rejection(agent.call(exec("@orders/behind")).await);

    assert_eq!(on_cluster.code, ErrorCode::INVALID_PARAMS);
    assert_eq!(through_cluster.code, ErrorCode::INVALID_PARAMS);
    assert!(approver.was_not_asked());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_personal_project_is_read_only() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;

    let put = ActionCall::new(ResourcePut::ID)
        .target("@personal/anything")
        .params(json!({ "kind": "host", "address": "10.0.3.3" }));
    let remove = ActionCall::new(ResourceRemove::ID).target("@personal/anything");
    let close = ActionCall::new(ProjectClose::ID).params(json!({ "id": "personal" }));

    let writing = rejection(app.call(put).await);
    let removing = rejection(app.call(remove).await);
    let closing = rejection(app.call(close).await);

    assert!(writing.message.contains("read-only"), "{}", writing.message);
    assert!(
        removing.message.contains("read-only"),
        "{}",
        removing.message
    );
    assert!(closing.message.contains("built in"), "{}", closing.message);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_personal_project_is_always_listed_first_and_read_only() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;

    let listed = app
        .call(ActionCall::new(ProjectList::ID))
        .await
        .unwrap()
        .value;

    assert_eq!(listed[0]["slug"], json!("personal"));
    assert_eq!(listed[0]["read_only"], json!(true));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_cannot_take_the_name_of_the_personal_project() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = harness.grounds().path().join("Personal");
    fs::create_dir(&folder).unwrap();

    let call = ActionCall::new(ProjectOpen::ID).params(json!({ "path": folder }));
    let error = rejection(app.call(call).await);

    assert!(
        error.message.contains("Personal project"),
        "{}",
        error.message
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn open_projects_are_reopened_from_the_state_database_after_a_restart()
-> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    let closed = harness.grounds().path().join("billing");
    fs::create_dir(&closed).unwrap();
    open(&app, &folder).await?;
    open(&app, &closed).await?;
    let close = ActionCall::new(ProjectClose::ID).params(json!({ "id": "billing" }));
    app.call(close).await.unwrap();

    let harness = harness.restart(core).await?;
    let app = harness.app().await?;
    let listed = app
        .call(ActionCall::new(ProjectList::ID))
        .await
        .unwrap()
        .value;
    let slugs: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|project| project["slug"].as_str().unwrap())
        .collect();

    assert_eq!(slugs, ["personal", "orders"]);
    assert_eq!(listed[1]["id"], json!(PROJECT_ID));
    assert_eq!(names(&app).await?, ["api-1", "bastion"]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_closed_project_is_no_longer_watched_or_reachable() -> Result<(), TestError> {
    let harness = start()?;
    let app = harness.app().await?;
    let folder = orders(&harness)?;
    open(&app, &folder).await?;

    let close = ActionCall::new(ProjectClose::ID).params(json!({ "id": PROJECT_ID }));
    app.call(close).await.unwrap();
    let call = ActionCall::new(ResourceList::ID).params(json!({ "project": "orders" }));
    let error = rejection(app.call(call).await);

    assert!(
        error.message.contains("no open project"),
        "{}",
        error.message
    );

    Ok(())
}
