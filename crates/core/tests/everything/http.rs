use serde_json::json;
use synt_journal::{Actor, EventKind, Name};
use synt_proto::{ActionCall, ApprovalDecision, ErrorCode, Requester, RiskLevel};

use super::Stage;
use crate::common::rejection;

fn every() -> Actor {
    Actor::Plugin(Name::new("every").unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_extension_reaches_a_listed_host_through_the_core() {
    let stage = Stage::start().await;

    let outer = stage
        .try_every("fetch", json!({ "url": stage.echo.url("/health") }))
        .await
        .unwrap();
    let requested = stage.nested("extension.http");
    let trail = stage.trail(requested[0].seq());

    assert_eq!(outer.value["status"], 200);
    assert_eq!(outer.value["body"], "GET /health auth= body=");
    assert_eq!(stage.echo.received()[0].path, "/health");

    assert_eq!(requested.len(), 1);
    assert_eq!(requested[0].actor(), &every());
    assert_eq!(
        requested[0].details().value()["parent"],
        json!(outer.request.0)
    );
    assert_eq!(
        requested[0].details().value()["params"]["url"],
        json!(stage.echo.url("/health"))
    );

    let finished = trail
        .iter()
        .find(|event| event.kind() == EventKind::Finished)
        .unwrap();
    assert_eq!(finished.details().value()["status"], 200);
    assert!(finished.details().value().get("body").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_core_inserts_a_secret_the_extension_never_sees() {
    let stage = Stage::start().await;
    stage.put_secret("test/token", "s3cr3t-value").await;

    let fetched = stage
        .every(
            "fetch",
            json!({ "url": stage.echo.url("/me"), "secret": "test/token" }),
        )
        .await;

    assert_eq!(
        stage.echo.received()[0].authorization.as_deref(),
        Some("Bearer s3cr3t-value")
    );
    assert_eq!(
        fetched["body"],
        "GET /me auth=Bearer secret://test/token body="
    );
    assert_eq!(fetched["echoed"], "Bearer secret://test/token");
    assert!(!fetched.to_string().contains("s3cr3t-value"));
    assert!(!stage.journal_text().contains("s3cr3t-value"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_secret_the_manifest_does_not_name_stays_home() {
    let stage = Stage::start().await;
    stage.put_secret("other/token", "not-for-you").await;

    let refused = stage
        .try_every(
            "fetch",
            json!({ "url": stage.echo.url("/me"), "secret": "other/token" }),
        )
        .await;

    let error = rejection(refused);
    assert!(
        error.message.contains("does not let secret://other/token"),
        "{}",
        error.message
    );
    assert!(stage.echo.received().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn hosts_outside_the_manifest_are_refused() {
    let stage = Stage::start().await;
    let elsewhere = format!("http://127.0.0.1:{}/", stage.echo.port() + 1);

    let error = rejection(stage.try_every("fetch", json!({ "url": elsewhere })).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(
        error.message.contains("not listed in capabilities.http"),
        "{}",
        error.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_request_that_writes_asks_the_person() {
    let mut stage = Stage::start().await;
    let app = stage.app().clone();
    let post = ActionCall::new("every.fetch").params(json!({
        "url": stage.echo.url("/deploy"),
        "method": "POST",
        "body": "now",
    }));

    let (question, outcome) = stage
        .approver
        .decide(&app, post, ApprovalDecision::AllowOnce)
        .await;

    assert_eq!(question.actor, Requester::Plugin("every".to_owned()));
    assert_eq!(question.on_behalf_of, Some(Requester::Person));
    assert_eq!(question.action, "extension.http");
    assert_eq!(question.risk, RiskLevel::Write);
    assert_eq!(question.params["body_bytes"], 3);
    assert_eq!(
        outcome.unwrap().value["body"],
        "POST /deploy auth= body=now"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn nobody_but_an_extension_sends_through_extension_http() {
    let stage = Stage::start().await;

    let error = rejection(
        stage
            .try_call(
                "extension.http",
                json!({ "method": "GET", "url": stage.echo.url("/") }),
            )
            .await,
    );

    assert!(
        error.message.contains("only an extension"),
        "{}",
        error.message
    );
    assert!(stage.echo.received().is_empty());
}
