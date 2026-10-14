#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{Grounds, Harness, kinds, rejection};
use serde_json::{Value, json};
use synt_core::{Broker, Core, Kube, Projects, ResourceList};
use synt_journal::EventKind;
use synt_kube::KubeconfigSource;
use synt_proto::{ActionCall, ActionDescription, ErrorCode, RiskLevel};
use synt_ssh::Pool;

const NOWHERE: &str = "https://127.0.0.1:1";

fn kubeconfig(grounds: &Grounds, text: &str) -> PathBuf {
    let path = grounds.path().join("kubeconfig");
    fs::write(&path, text).unwrap();

    path
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

fn harness(text: &str) -> Harness {
    let grounds = Grounds::new();
    let path = kubeconfig(&grounds, text);
    let projects = Projects::start_with(grounds.state(), KubeconfigSource::File(path)).unwrap();

    let mut registry = grounds.registry();
    registry
        .register(ResourceList::new(projects.clone()))
        .unwrap();
    Kube::new(Pool::builder().build(), projects, grounds.journal())
        .register(&mut registry)
        .unwrap();

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
async fn every_cluster_action_is_described_with_its_risk_and_params() {
    let harness = harness(&two_contexts());
    let agent = harness.agent("deploy-bot").await;

    let described = agent.describe().await.unwrap();
    let rules: Vec<(&str, RiskLevel, bool)> = [
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
            ("k8s.contexts", RiskLevel::Read, false),
            ("k8s.list", RiskLevel::Read, true),
            ("k8s.get", RiskLevel::Read, true),
            ("k8s.events", RiskLevel::Read, true),
            ("k8s.logs", RiskLevel::Read, true),
            ("k8s.watch", RiskLevel::Read, true),
            ("k8s.stream.close", RiskLevel::Read, false),
            ("k8s.apply", RiskLevel::Write, true),
            ("k8s.scale", RiskLevel::Write, true),
            ("k8s.rollout_restart", RiskLevel::Write, true),
            ("k8s.delete", RiskLevel::Write, true),
            ("k8s.exec", RiskLevel::Write, true),
            ("k8s.port_forward", RiskLevel::Write, true),
            ("k8s.port_forward.list", RiskLevel::Read, false),
            ("k8s.port_forward.close", RiskLevel::Write, false),
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
}

#[tokio::test(flavor = "multi_thread")]
async fn kubeconfig_contexts_are_listed_without_their_credentials() {
    let harness = harness(&two_contexts());
    let agent = harness.agent("deploy-bot").await;

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
}

#[tokio::test(flavor = "multi_thread")]
async fn a_broken_kubeconfig_is_a_problem_of_the_personal_project_that_quotes_nothing() {
    let harness = harness("users:\n- name: a\n  user:\n    token: leaked-token\n    bad: [\n");
    let agent = harness.agent("deploy-bot").await;

    let personal = agent
        .call(ActionCall::new("resource.list").params(json!({ "project": "personal" })))
        .await
        .unwrap();
    let problems = personal.value["problems"].to_string();

    assert!(problems.contains("kubeconfig"), "{problems}");
    assert!(!problems.contains("leaked-token"), "{problems}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_context_is_refused_before_anyone_is_asked() {
    let harness = harness(&two_contexts());
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let refused = rejection(
        agent
            .call(call(
                "nowhere",
                "k8s.delete",
                json!({ "kind": "pods", "name": "api" }),
            ))
            .await,
    );
    let trail = kinds(&harness.events());

    assert_eq!(refused.code, ErrorCode::ACTION_FAILED);
    assert!(
        refused.message.contains("no context \"nowhere\""),
        "{}",
        refused.message
    );
    assert!(approver.was_not_asked());
    assert_eq!(trail, [EventKind::Requested, EventKind::Failed]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_that_does_not_answer_fails_before_anyone_is_asked() {
    let harness = harness(&two_contexts());
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;
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
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_params_are_refused_with_the_reason() {
    let harness = harness(&two_contexts());
    let agent = harness.agent("deploy-bot").await;

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
}

#[tokio::test(flavor = "multi_thread")]
async fn closing_what_is_not_open_fails() {
    let harness = harness(&two_contexts());
    let app = harness.app().await;

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
}
