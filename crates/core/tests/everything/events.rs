use serde_json::{Value, json};
use synt_proto::{ActionCall, ApprovalDecision, ErrorCode};
use synt_testkit::TestError;

use super::Stage;
use synt_testkit::core::{eventually, rejection};

async fn heard(stage: &Stage) -> Result<Vec<Value>, TestError> {
    let heard = stage.every("heard", json!({})).await?;

    Ok(heard["events"].as_array().cloned().unwrap_or_default())
}

fn by_extension(event: &Value) -> bool {
    event["payload"]["actor"] == json!({ "type": "plugin", "name": "every" })
}

#[tokio::test(flavor = "multi_thread")]
async fn the_extension_hears_the_journal_but_not_its_own_calls() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    stage.call("core.ping", json!({})).await?;
    stage.every("ping", json!({})).await?;

    let events = eventually("the ping in the journal", async || {
        let events = heard(&stage).await.ok()?;

        events
            .iter()
            .any(|event| event["kind"] == "journal" && event["payload"]["action"] == "core.ping")
            .then_some(events)
    })
    .await;

    assert!(events.iter().all(|event| !by_extension(event)));
    assert!(stage.nested("core.ping")?.len() >= 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_extension_hears_approval_outcomes() -> Result<(), TestError> {
    let mut stage = Stage::start().await?;
    let agent = stage.harness.agent("helper").await?;

    let (_, outcome) = stage
        .approver
        .decide(
            &agent,
            ActionCall::new("test.touch"),
            ApprovalDecision::Deny,
        )
        .await?;
    assert_eq!(rejection(outcome).code, ErrorCode::DENIED);

    let denial = eventually("the denial", async || {
        heard(&stage)
            .await
            .ok()?
            .into_iter()
            .find(|event| event["kind"] == "approval")
    })
    .await;

    assert_eq!(denial["payload"]["outcome"], "denied");
    assert_eq!(denial["payload"]["action"], "test.touch");
    assert_eq!(
        denial["payload"]["actor"],
        json!({ "type": "agent", "name": "helper" })
    );

    Ok(())
}
