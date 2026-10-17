#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::sync::Arc;

use common::{ExecStub, Grounds, Harness, rejection};
use serde_json::{Value, json};
use synt_actions::Action;
use synt_core::{Broker, Core, CoreCaller, CoreScreen};
use synt_extensions::{Extensions, Remote, Runtime};
use synt_journal::{Actor, EventKind, Filter, Name, ResourceRef, Seq};
use synt_proto::{ActionCall, ApprovalDecision, ErrorCode, Risk};

const SAMPLE: &str = include_str!("../../testkit/fixtures/sample.wat");
const MANIFEST: &str = r#"
id = "t"
name = "Test"
version = "0.1.0"
api = "0.1"
description = "Calls ssh.exec through the core."

[capabilities]
call = ["ssh.exec"]
"#;

const UNUSED_REGISTRY: &str = "http://127.0.0.1:9";
const UNUSED_KEY: &str = "RWQOuk/x7vRoqs2E6Vxffy293iLd8y+C5Yo2oilreJ2V22RL3qFeV7f2";
const HOST: &str = "synt-test";

async fn start() -> Harness {
    let grounds = Grounds::new();
    let mut registry = grounds.registry();
    registry.register(ExecStub).unwrap();

    let extensions = Extensions::open(
        &grounds.path().join("data"),
        &grounds.state_path(),
        Runtime::new().unwrap(),
        Remote::new(UNUSED_REGISTRY, UNUSED_KEY).unwrap(),
    )
    .unwrap();
    extensions.register(&mut registry).unwrap();

    let core = Arc::new(Core::new(registry, grounds.journal(), Broker::new()));
    extensions.connect(
        Arc::new(CoreCaller::new(&core)),
        Arc::new(CoreScreen::new(&core)),
    );

    let harness = Harness::serve(grounds, core);
    install(&harness).await;

    harness
}

async fn install(harness: &Harness) {
    let build = harness.grounds().path().join("build");
    fs::create_dir(&build).unwrap();
    fs::write(build.join("extension.toml"), MANIFEST).unwrap();
    fs::write(
        build.join("extension.wasm"),
        wat::parse_str(SAMPLE).unwrap(),
    )
    .unwrap();

    let mut approver = harness.approver().await;
    let app = approver.app().clone();
    let call = ActionCall::new("extension.install_local").params(json!({ "path": build }));

    approver
        .approve(&app, call, ApprovalDecision::AllowOnce)
        .await;
}

fn through_extension(params: Value) -> ActionCall {
    ActionCall::new("t.call").target(HOST).params(params)
}

fn uptime() -> ActionCall {
    through_extension(json!({ "command": "uptime" }))
}

fn extension() -> Actor {
    Actor::Plugin(Name::new("t").unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_is_asked_before_an_extension_writes_on_their_behalf() {
    let harness = start().await;
    let mut approver = harness.approver().await;
    let app = approver.app().clone();

    let (question, outcome) = approver
        .decide(&app, uptime(), ApprovalDecision::AllowOnce)
        .await;

    assert_eq!(question.actor, Actor::Plugin(Name::new("t").unwrap()));
    assert_eq!(question.on_behalf_of, Some(Actor::Person));
    assert_eq!(question.action, ExecStub::ID);
    assert_eq!(
        question.target.as_ref().map(ResourceRef::as_str),
        Some(HOST)
    );
    assert_eq!(
        question.destination.as_deref(),
        Some("synt-test = root@203.0.113.7:22")
    );
    assert_eq!(question.risk, Risk::Write);
    assert_eq!(question.params, json!({ "command": "uptime" }));
    assert_eq!(
        outcome.unwrap().value,
        json!({ "exit": 0, "stdout": ExecStub::STDOUT })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_nested_call_is_journaled_as_the_extension_under_the_call_that_made_it() {
    let harness = start().await;
    let mut approver = harness.approver().await;
    let app = approver.app().clone();

    let outer = approver
        .approve(&app, uptime(), ApprovalDecision::AllowOnce)
        .await;
    let outer_seq = Seq::new(outer.request.get());
    let nested = harness
        .matching(&Filter::all().kind(EventKind::Requested))
        .into_iter()
        .find(|event| event.actor() == &extension())
        .unwrap();
    let nested_trail = harness.matching(&Filter::all().call(nested.seq()));
    let outer_trail = harness.trail(outer.request);

    assert!(nested.seq() > outer_seq);
    assert_eq!(nested.action().as_str(), ExecStub::ID);
    assert_eq!(nested.resource().map(ResourceRef::as_str), Some(HOST));
    assert_eq!(
        nested.details().value(),
        &json!({ "params": { "command": "uptime" }, "parent": outer_seq.get() })
    );
    assert!(
        nested_trail
            .iter()
            .all(|event| event.actor() == &extension())
    );
    assert!(
        nested_trail
            .iter()
            .any(|event| event.kind() == EventKind::Finished)
    );
    assert!(
        outer_trail
            .iter()
            .all(|event| event.actor() == &Actor::Person)
    );
    assert_eq!(outer_trail.last().unwrap().kind(), EventKind::Finished);
    assert!(harness.is_intact());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_denied_nested_call_fails_the_call_that_made_it() {
    let harness = start().await;
    let mut approver = harness.approver().await;
    let app = approver.app().clone();

    let (_, outcome) = approver
        .decide(&app, uptime(), ApprovalDecision::Deny)
        .await;
    let error = rejection(outcome);
    let denied = harness.matching(&Filter::all().kind(EventKind::Denied));

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(error.message.contains("denied"), "{}", error.message);
    assert_eq!(denied.len(), 1);
    assert_eq!(denied[0].actor(), &extension());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_extension_may_be_granted_writes_until_restart() {
    let harness = start().await;
    let mut approver = harness.approver().await;
    let app = approver.app().clone();

    approver
        .approve(&app, uptime(), ApprovalDecision::AllowUntilRestart)
        .await;
    let again = app.call(uptime()).await;

    assert!(again.is_ok());
    assert!(approver.was_not_asked());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_extension_is_asked_for_every_privileged_call() {
    let harness = start().await;
    let mut approver = harness.approver().await;
    let app = approver.app().clone();
    let privileged = || through_extension(json!({ "command": "reboot", "privileged": true }));

    let (first, _) = approver
        .decide(&app, privileged(), ApprovalDecision::AllowUntilRestart)
        .await;
    let (second, outcome) = approver
        .decide(&app, privileged(), ApprovalDecision::AllowOnce)
        .await;

    assert_eq!(first.risk, Risk::Privileged);
    assert_eq!(second.risk, Risk::Privileged);
    assert!(second.id > first.id);
    assert!(outcome.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_using_an_extension_is_asked_for_both_calls() {
    let harness = start().await;
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let running = tokio::spawn(async move { agent.call(uptime()).await });
    let outer = approver.next().await;
    approver
        .answer(&outer, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let nested = approver.next().await;
    approver
        .answer(&nested, ApprovalDecision::AllowOnce)
        .await
        .unwrap();
    let outcome = running.await.unwrap();

    assert_eq!(outer.actor, Actor::Agent(Name::new("deploy-bot").unwrap()));
    assert_eq!(outer.action, "t.call");
    assert_eq!(nested.actor, Actor::Plugin(Name::new("t").unwrap()));
    assert_eq!(
        nested.on_behalf_of,
        Some(Actor::Agent(Name::new("deploy-bot").unwrap()))
    );
    assert!(outcome.is_ok());
}
