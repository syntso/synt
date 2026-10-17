#![allow(clippy::unwrap_used)]

mod common;

use common::{Guarded, Harness, Secret, kinds, rejection};
use serde_json::json;
use synt_actions::Action;
use synt_journal::EventKind;
use synt_proto::{ActionCall, ActionDescription, ErrorCode, Risk};

fn find<'a>(descriptions: &'a [ActionDescription], id: &str) -> Option<&'a ActionDescription> {
    descriptions.iter().find(|description| description.id == id)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_is_shown_every_action_with_its_rules() {
    let harness = Harness::start();
    let app = harness.app().await;

    let described = app.describe().await.unwrap();
    let secret = find(&described, Secret::ID).unwrap();
    let guarded = find(&described, Guarded::ID).unwrap();

    assert!(secret.person_only);
    assert_eq!(secret.title, Secret::TITLE);
    assert!(!guarded.person_only);
    assert_eq!(guarded.risk, Risk::Read);
    assert!(!guarded.takes_target);
    assert_eq!(guarded.params["type"], json!("object"));
    assert!(guarded.params["properties"]["risk"].is_object());
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_are_not_shown_actions_for_the_person_only() {
    let harness = Harness::start();
    let app = harness.app().await;
    let agent = harness.agent("deploy-bot").await;

    let for_person = app.describe().await.unwrap();
    let for_agent = agent.describe().await.unwrap();

    assert!(find(&for_agent, Secret::ID).is_none());
    assert!(for_agent.iter().all(|description| !description.person_only));
    assert_eq!(
        for_agent.len(),
        for_person
            .iter()
            .filter(|description| !description.person_only)
            .count()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn describing_leaves_no_trace_in_the_journal() {
    let harness = Harness::start();
    let app = harness.app().await;

    app.describe().await.unwrap();

    assert!(harness.events().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_calling_a_hidden_action_is_denied_and_journaled() {
    let harness = Harness::start();
    let agent = harness.agent("deploy-bot").await;

    let error = rejection(agent.call(ActionCall::new(Secret::ID)).await);
    let trail = harness.events();

    assert_eq!(error.code, ErrorCode::DENIED);
    assert_eq!(error.message, "test.secret is for the person only");
    assert_eq!(kinds(&trail), [EventKind::Requested, EventKind::Denied]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_calls_a_person_only_action_without_being_asked() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;

    approver
        .app()
        .call(ActionCall::new(Secret::ID))
        .await
        .unwrap();

    assert!(approver.was_not_asked());
}
