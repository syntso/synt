use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use synt_core::{Broker, Core, Kube, ProjectOpen, Projects, ResourceList};
use synt_journal::EventKind;
use synt_kube::KubeconfigSource;
use synt_proto::{ActionCall, ActionDescription, ErrorCode, Risk};
use synt_ssh::Pool;
use synt_testkit::TestError;
use synt_testkit::core::{Grounds, Harness, kinds, rejection};

const NOWHERE: &str = "https://127.0.0.1:1";

fn kubeconfig(grounds: &Grounds, text: &str) -> Result<PathBuf, TestError> {
    let path = grounds.path().join("kubeconfig");
    fs::write(&path, text)?;

    Ok(path)
}

fn two_contexts() -> String {
    format!(
        "apiVersion: v1
kind: Config
current-context: first
clusters:
- name: closed
  cluster:
    server: {NOWHERE}
users:
- name: someone
  user:
    token: never-shown-token
contexts:
- name: first
  context:
    cluster: closed
    user: someone
    namespace: orders
- name: arn:aws:eks:eu-west-1:1:cluster/second
  context:
    cluster: closed
    user: someone
"
    )
}

fn harness(text: &str) -> Result<Harness, TestError> {
    let grounds = Grounds::new()?;
    let path = kubeconfig(&grounds, text)?;
    let projects = Projects::start_with(grounds.state()?, KubeconfigSource::File(path))?;

    let mut registry = grounds.registry()?;
    registry.register(ResourceList::new(projects.clone()))?;
    registry.register(ProjectOpen::new(projects.clone()))?;
    Kube::new(Pool::builder().build(), projects, grounds.journal()).register(&mut registry)?;

    let core = Core::new(registry, grounds.journal(), Broker::new());

    Harness::serve(grounds, core)
}

fn find<'a>(described: &'a [ActionDescription], id: &str) -> &'a ActionDescription {
    described
        .iter()
        .find(|description| description.id == id)
        .unwrap_or_else(|| panic!("{id} is described"))
}

fn call(target: &str, action: &str, params: Value) -> ActionCall {
    ActionCall::new(action).target(target).params(params)
}

#[tokio::test(flavor = "multi_thread")]
async fn every_cluster_action_is_described_with_its_risk_and_params() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let agent = harness.agent("deploy-bot").await?;

    let described = agent.describe().await.unwrap();
    let rules: Vec<(&str, Risk, bool)> = [
        "k8s.contexts",
        "k8s.list",
        "k8s.get",
        "k8s.events",
        "k8s.logs",
        "k8s.watch",
        "k8s.stream.close",
        "k8s.apply",
        "k8s.scale",
        "k8s.rollout_restart",
        "k8s.delete",
        "k8s.exec",
        "k8s.port_forward",
        "k8s.port_forward.list",
        "k8s.port_forward.close",
    ]
    .iter()
    .map(|id| {
        let description = find(&described, id);
        (*id, description.risk, description.takes_target)
    })
    .collect();
    let list = find(&described, "k8s.list");

    assert_eq!(
        rules,
        [
            ("k8s.contexts", Risk::Read, false),
            ("k8s.list", Risk::Read, true),
            ("k8s.get", Risk::Read, true),
            ("k8s.events", Risk::Read, true),
            ("k8s.logs", Risk::Read, true),
            ("k8s.watch", Risk::Read, true),
            ("k8s.stream.close", Risk::Read, false),
            ("k8s.apply", Risk::Write, true),
            ("k8s.scale", Risk::Write, true),
            ("k8s.rollout_restart", Risk::Write, true),
            ("k8s.delete", Risk::Write, true),
            ("k8s.exec", Risk::Write, true),
            ("k8s.port_forward", Risk::Write, true),
            ("k8s.port_forward.list", Risk::Read, false),
            ("k8s.port_forward.close", Risk::Write, false),
        ]
    );
    assert!(!list.title.is_empty());
    assert_eq!(list.params["required"], json!(["kind"]));
    for field in [
        "kind",
        "namespace",
        "all_namespaces",
        "selector",
        "limit",
        "continue",
    ] {
        assert!(list.params["properties"][field].is_object(), "{field}");
    }
    assert!(list.params["properties"]["facts"].is_null());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn kubeconfig_contexts_are_listed_without_their_credentials() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let agent = harness.agent("deploy-bot").await?;

    let contexts = agent.call(ActionCall::new("k8s.contexts")).await.unwrap();
    let personal = agent
        .call(ActionCall::new("resource.list").params(json!({ "project": "personal" })))
        .await
        .unwrap();

    let clusters: Vec<&Value> = personal.value["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["kind"] == "cluster")
        .collect();

    assert_eq!(contexts.value[0]["name"], "first");
    assert_eq!(contexts.value[0]["namespace"], "orders");
    assert_eq!(contexts.value[0]["current"], true);
    assert_eq!(contexts.value[1]["server"], NOWHERE);
    assert!(!contexts.value.to_string().contains("never-shown-token"));
    assert_eq!(clusters[0]["name"], "first");
    assert_eq!(clusters[0]["context"], "first");
    assert_eq!(
        clusters[1]["name"],
        "arn-aws-eks-eu-west-1-1-cluster-second"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_kubeconfig_is_a_problem_of_the_personal_project_that_quotes_nothing()
-> Result<(), TestError> {
    let harness = harness("users:\n- name: a\n  user:\n    token: leaked-token\n    bad: [\n")?;
    let agent = harness.agent("deploy-bot").await?;

    let personal = agent
        .call(ActionCall::new("resource.list").params(json!({ "project": "personal" })))
        .await
        .unwrap();
    let problems = personal.value["problems"].to_string();

    assert!(problems.contains("kubeconfig"), "{problems}");
    assert!(!problems.contains("leaked-token"), "{problems}");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_context_is_refused_before_anyone_is_asked() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;

    let refused = rejection(
        agent
            .call(call(
                "nowhere",
                "k8s.delete",
                json!({ "kind": "pods", "name": "api" }),
            ))
            .await,
    );
    let trail = kinds(&harness.events()?);

    assert_eq!(refused.code, ErrorCode::ACTION_FAILED);
    assert!(
        refused.message.contains("no context \"nowhere\""),
        "{}",
        refused.message
    );
    assert!(approver.was_not_asked());
    assert_eq!(trail, [EventKind::Requested, EventKind::Failed]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_that_does_not_answer_fails_before_anyone_is_asked() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let mut approver = harness.approver().await?;
    let agent = harness.agent("deploy-bot").await?;
    let manifest = "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: settings\n";

    let deleting = rejection(
        agent
            .call(call(
                "first",
                "k8s.delete",
                json!({ "kind": "pods", "name": "api" }),
            ))
            .await,
    );
    let applying = rejection(
        agent
            .call(call("first", "k8s.apply", json!({ "manifest": manifest })))
            .await,
    );

    assert_eq!(deleting.code, ErrorCode::ACTION_FAILED);
    assert!(
        deleting.message.contains("cannot connect"),
        "{}",
        deleting.message
    );
    assert_eq!(applying.code, ErrorCode::ACTION_FAILED);
    assert!(approver.was_not_asked());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_params_are_refused_with_the_reason() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let agent = harness.agent("deploy-bot").await?;

    let cases = [
        (
            "k8s.get",
            json!({ "kind": "pods", "name": "a/../../secrets" }),
        ),
        ("k8s.get", json!({ "kind": "pods/exec", "name": "api" })),
        ("k8s.list", json!({ "kind": "pods", "namespace": "Orders" })),
        (
            "k8s.list",
            json!({ "kind": "pods", "namespace": "a", "all_namespaces": true }),
        ),
        ("k8s.logs", json!({ "pod": "api", "follow": true })),
        ("k8s.exec", json!({ "pod": "api", "command": [] })),
        (
            "k8s.exec",
            json!({ "pod": "api", "command": ["ls"], "timeout_seconds": 0 }),
        ),
        (
            "k8s.port_forward",
            json!({ "pod": "api", "service": "web", "port": 80 }),
        ),
        ("k8s.port_forward", json!({ "pod": "api", "port": 0 })),
        ("k8s.apply", json!({ "manifest": "kind: [" })),
        (
            "k8s.apply",
            json!({ "manifest": "apiVersion: v1\nkind: Pod\n" }),
        ),
    ];

    for (action, params) in cases {
        let refused = rejection(agent.call(call("first", action, params.clone())).await);

        assert_eq!(
            refused.code,
            ErrorCode::INVALID_PARAMS,
            "{action} {params}: {}",
            refused.message
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_what_is_not_open_fails() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let app = harness.app().await?;

    let stream = rejection(
        app.call(ActionCall::new("k8s.stream.close").params(json!({ "request": 999 })))
            .await,
    );
    let forward = rejection(
        app.call(ActionCall::new("k8s.port_forward.close").params(json!({ "id": 7 })))
            .await,
    );

    assert_eq!(stream.code, ErrorCode::ACTION_FAILED);
    assert!(stream.message.contains("999"), "{}", stream.message);
    assert_eq!(forward.code, ErrorCode::ACTION_FAILED);
    assert!(
        forward.message.contains("no open port forward 7"),
        "{}",
        forward.message
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kubeconfig_inside_a_project_folder_is_never_read() -> Result<(), TestError> {
    let harness = harness(&two_contexts())?;
    let app = harness.app().await?;
    let folder = harness.grounds().path().join("shop");
    let marker = harness.grounds().path().join("ran");
    let planted = folder.join("extensions/notes/kube.yaml");
    fs::create_dir_all(planted.parent().unwrap()).unwrap();
    fs::create_dir_all(folder.join("resources")).unwrap();
    fs::write(
        folder.join("synt.toml"),
        "id = \"5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e\"\n",
    )
    .unwrap();
    fs::write(
        &planted,
        format!(
            "apiVersion: v1\nkind: Config\ncurrent-context: c\nclusters:\n- name: k\n  cluster:\n    \
             server: {NOWHERE}\nusers:\n- name: u\n  user:\n    exec:\n      apiVersion: \
             client.authentication.k8s.io/v1\n      command: touch\n      args: [\"{}\"]\n\
             contexts:\n- name: c\n  context:\n    cluster: k\n    user: u\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::write(
        folder.join("resources/kube.toml"),
        format!(
            "kind = \"cluster\"\nkubeconfig = \"{}\"\n",
            folder.join("extensions/notes/../notes/kube.yaml").display()
        ),
    )
    .unwrap();
    app.call(ActionCall::new("project.open").params(json!({ "path": folder })))
        .await
        .unwrap();

    let refused = rejection(
        app.call(call("@shop/kube", "k8s.list", json!({ "kind": "pods" })))
            .await,
    );

    assert!(
        refused.message.contains("inside the project folder"),
        "{}",
        refused.message
    );
    assert!(!marker.exists());

    Ok(())
}
