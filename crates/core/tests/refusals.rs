#![allow(clippy::unwrap_used)]

mod common;

use common::{Failing, Harness, Panicking, Touch, kinds, ping, rejection};
use serde_json::json;
use synt_actions::Action;
use synt_journal::{ActionId, EventKind};
use synt_proto::{ActionCall, ErrorCode, RequestSeq};

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_action_is_requested_then_failed() {
    let harness = Harness::start();
    let client = harness.app().await;

    let error = rejection(client.call(ActionCall::new("test.missing")).await);
    let trail = harness.events();

    assert_eq!(error.code, ErrorCode::UNKNOWN_ACTION);
    assert_eq!(kinds(&trail), [EventKind::Requested, EventKind::Failed]);
    assert_eq!(trail[1].call(), Some(trail[0].seq()));
    assert_eq!(trail[1].action(), &ActionId::new("test.missing").unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_write_action_is_denied_without_running() {
    let harness = Harness::start();
    let client = harness.app().await;

    let error = rejection(client.call(ActionCall::new(Touch::ID)).await);
    let trail = harness.events();

    assert_eq!(error.code, ErrorCode::DENIED);
    assert_eq!(harness.touches(), 0);
    assert_eq!(kinds(&trail), [EventKind::Requested, EventKind::Denied]);
    assert_eq!(trail[1].call(), Some(trail[0].seq()));
    assert_eq!(
        trail[1].details().value(),
        &json!({ "error": error.message })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_are_denied_writes_as_well() {
    let harness = Harness::start();
    let client = harness.agent("deploy-bot").await;

    let error = rejection(client.call(ActionCall::new(Touch::ID)).await);

    assert_eq!(error.code, ErrorCode::DENIED);
    assert_eq!(harness.touches(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_action_is_journaled_as_failed() {
    let harness = Harness::start();
    let client = harness.app().await;

    let error = rejection(client.call(ActionCall::new(Failing::ID)).await);
    let trail = harness.events();

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert_eq!(error.message, Failing::REASON);
    assert_eq!(
        kinds(&trail),
        [EventKind::Requested, EventKind::Started, EventKind::Failed]
    );
    assert_eq!(trail[2].call(), Some(trail[0].seq()));
    assert_eq!(
        trail[2].details().value(),
        &json!({ "error": Failing::REASON })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panicking_action_is_failed_and_the_connection_survives() {
    let harness = Harness::start();
    let client = harness.app().await;

    let error = rejection(client.call(ActionCall::new(Panicking::ID)).await);
    let trail = harness.events();
    let after = client.call(ping()).await;

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert_eq!(
        kinds(&trail),
        [EventKind::Requested, EventKind::Started, EventKind::Failed]
    );
    assert!(after.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn params_the_action_cannot_read_fail_after_starting() {
    let harness = Harness::start();
    let client = harness.app().await;

    let call = ping().params(json!({ "unexpected": true }));
    let error = rejection(client.call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert_eq!(
        kinds(&harness.events()),
        [EventKind::Requested, EventKind::Started, EventKind::Failed]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn params_that_cannot_be_recorded_leave_no_trace() {
    let harness = Harness::start();
    let client = harness.app().await;

    let call = ping().params(json!({ "ratio": 0.5 }));
    let error = rejection(client.call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(harness.events().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_action_name_leaves_no_trace() {
    let harness = Harness::start();
    let client = harness.app().await;

    let error = rejection(client.call(ActionCall::new("core.ping\nrm -rf")).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(harness.events().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refusal_leaves_the_connection_usable() {
    let harness = Harness::start();
    let client = harness.app().await;

    let refused = rejection(client.call(ActionCall::new(Touch::ID)).await);
    let outcome = client.call(ping()).await.unwrap();

    assert_eq!(refused.code, ErrorCode::DENIED);
    assert_eq!(outcome.request, RequestSeq(3));
    assert!(harness.is_intact());
}
