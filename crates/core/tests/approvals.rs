#![allow(clippy::unwrap_used)]

mod common;

use std::time::Duration;

use common::{Guarded, Harness, eventually, kinds, rejection};
use serde_json::json;
use synt_actions::Action;
use synt_core::Broker;
use synt_journal::{EventKind, Filter};
use synt_proto::{ApprovalDecision, ApprovalId, ErrorCode, Requester, RiskLevel};

const SHORT_PATIENCE: Duration = Duration::from_millis(300);

#[tokio::test(flavor = "multi_thread")]
async fn reads_run_for_everyone_without_asking() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    approver.app().call(Guarded::call("read")).await.unwrap();
    agent.call(Guarded::call("read")).await.unwrap();

    assert!(approver.was_not_asked());
    assert_eq!(harness.runs(), 2);
    assert!(!kinds(&harness.events()).contains(&EventKind::Approved));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_writes_without_being_asked() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;

    let outcome = approver.app().call(Guarded::call("write")).await.unwrap();

    assert!(approver.was_not_asked());
    assert_eq!(
        kinds(&harness.trail(outcome.request)),
        [
            EventKind::Requested,
            EventKind::Started,
            EventKind::Finished
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_write_waits_for_the_person_and_runs_once_allowed() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let (question, outcome) = approver
        .decide(&agent, Guarded::call("write"), ApprovalDecision::AllowOnce)
        .await;
    let trail = harness.trail(outcome.unwrap().request);

    assert_eq!(question.actor, Requester::Agent("deploy-bot".to_owned()));
    assert_eq!(question.action, Guarded::ID);
    assert_eq!(question.risk, RiskLevel::Write);
    assert_eq!(question.params, json!({ "risk": "write" }));
    assert_eq!(question.on_behalf_of, None);
    assert_eq!(harness.runs(), 1);
    assert_eq!(
        kinds(&trail),
        [
            EventKind::Requested,
            EventKind::Approved,
            EventKind::Started,
            EventKind::Finished
        ]
    );
    assert_eq!(
        trail[1].details().value(),
        &json!({ "scope": "once", "by": "person" })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_person_is_asked_before_a_privileged_call() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let app = approver.app().clone();

    let (question, outcome) = approver
        .decide(
            &app,
            Guarded::call("privileged"),
            ApprovalDecision::AllowOnce,
        )
        .await;

    assert_eq!(question.actor, Requester::Person);
    assert_eq!(question.risk, RiskLevel::Privileged);
    assert!(outcome.is_ok());
    assert_eq!(harness.runs(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_denied_call_is_journaled_and_never_runs() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let (_, outcome) = approver
        .decide(&agent, Guarded::call("write"), ApprovalDecision::Deny)
        .await;
    let error = rejection(outcome);
    let trail = harness.events();

    assert_eq!(error.code, ErrorCode::DENIED);
    assert_eq!(error.message, "the person denied the request");
    assert_eq!(harness.runs(), 0);
    assert_eq!(kinds(&trail), [EventKind::Requested, EventKind::Denied]);
    assert_eq!(
        trail[1].details().value(),
        &json!({ "reason": "the person denied the request" })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_connected_app_an_agent_write_is_denied_at_once() {
    let harness = Harness::start();
    let agent = harness.agent("deploy-bot").await;

    let error = rejection(agent.call(Guarded::call("write")).await);

    assert_eq!(error.code, ErrorCode::DENIED);
    assert_eq!(error.message, "no one is available to approve");
    assert_eq!(harness.runs(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_are_never_asked_to_approve() {
    let harness = Harness::start();
    let watcher = harness.agent("watcher").await;
    let mut seen = watcher.notifications();
    let agent = harness.agent("deploy-bot").await;

    let error = rejection(agent.call(Guarded::call("write")).await);

    assert_eq!(error.message, "no one is available to approve");
    assert!(seen.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unanswered_question_is_denied_when_patience_runs_out() {
    let harness = Harness::with_broker(Broker::new().with_patience(SHORT_PATIENCE));
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let call = tokio::spawn(async move { agent.call(Guarded::call("write")).await });
    let question = approver.next().await;
    let error = rejection(call.await.unwrap());
    let late = rejection(
        approver
            .answer(&question, ApprovalDecision::AllowOnce)
            .await,
    );

    assert_eq!(error.code, ErrorCode::DENIED);
    assert_eq!(error.message, "approval timed out");
    assert_eq!(late.code, ErrorCode::APPROVAL_UNKNOWN);
    assert_eq!(harness.runs(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn every_app_is_asked_and_the_first_answer_wins() {
    let harness = Harness::start();
    let mut first = harness.approver().await;
    let mut second = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let call = tokio::spawn(async move { agent.call(Guarded::call("write")).await });
    let asked_first = first.next().await;
    let asked_second = second.next().await;

    first
        .answer(&asked_first, ApprovalDecision::Deny)
        .await
        .unwrap();
    let overruled = rejection(
        second
            .answer(&asked_second, ApprovalDecision::AllowOnce)
            .await,
    );
    let error = rejection(call.await.unwrap());

    assert_eq!(asked_first, asked_second);
    assert_eq!(overruled.code, ErrorCode::APPROVAL_UNKNOWN);
    assert_eq!(error.message, "the person denied the request");
    assert_eq!(harness.runs(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_cannot_answer_even_its_own_question() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let caller = agent.clone();
    let call = tokio::spawn(async move { caller.call(Guarded::call("write")).await });
    let question = approver.next().await;

    let refused = rejection(
        agent
            .respond(question.id, ApprovalDecision::AllowOnce)
            .await,
    );
    approver
        .answer(&question, ApprovalDecision::Deny)
        .await
        .unwrap();
    let error = rejection(call.await.unwrap());

    assert_eq!(refused.code, ErrorCode::DENIED);
    assert_eq!(error.message, "the person denied the request");
    assert_eq!(harness.runs(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_to_no_question_is_unknown() {
    let harness = Harness::start();
    let approver = harness.approver().await;

    let refused = rejection(
        approver
            .app()
            .respond(ApprovalId(404), ApprovalDecision::AllowOnce)
            .await,
    );

    assert_eq!(refused.code, ErrorCode::APPROVAL_UNKNOWN);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_question_is_withdrawn_when_its_requester_leaves() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let call = tokio::spawn(async move { agent.call(Guarded::call("write")).await });
    let question = approver.next().await;
    call.abort();
    let _ = call.await;

    let denied = eventually("the call to be denied", async || {
        harness
            .matching(&Filter::all().kind(EventKind::Denied))
            .pop()
    })
    .await;
    let late = rejection(
        approver
            .answer(&question, ApprovalDecision::AllowOnce)
            .await,
    );

    assert_eq!(
        denied.details().value(),
        &json!({ "reason": "the requester disconnected" })
    );
    assert_eq!(late.code, ErrorCode::APPROVAL_UNKNOWN);
    assert_eq!(harness.runs(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn questions_are_numbered_from_one() {
    let harness = Harness::start();
    let mut approver = harness.approver().await;
    let agent = harness.agent("deploy-bot").await;

    let (first, _) = approver
        .decide(&agent, Guarded::call("write"), ApprovalDecision::Deny)
        .await;
    let (second, _) = approver
        .decide(&agent, Guarded::call("write"), ApprovalDecision::Deny)
        .await;

    assert_eq!(first.id, ApprovalId(1));
    assert_eq!(second.id, ApprovalId(2));
}
