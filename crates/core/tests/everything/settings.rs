use serde_json::json;
use synt_proto::{ActionCall, ErrorCode};
use synt_testkit::TestError;

use super::Stage;
use synt_testkit::core::rejection;

#[tokio::test(flavor = "multi_thread")]
async fn the_person_fills_the_settings_form_and_the_extension_reads_it() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let form = stage
        .call("extension.settings.get", json!({ "extension": "every" }))
        .await?;
    stage
        .call(
            "extension.settings.set",
            json!({ "extension": "every", "values": { "greeting": "hi", "every": 5 } }),
        )
        .await?;

    assert_eq!(form["schema"]["properties"]["every"]["type"], "integer");
    assert_eq!(form["values"], json!({}));
    assert_eq!(
        stage.every("settings", json!({})).await?,
        json!({ "greeting": "hi", "every": 5 })
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn saved_values_never_reach_the_journal_only_their_keys() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let saved = stage
        .harness
        .app()
        .await?
        .call(ActionCall::new("extension.settings.set").params(
            json!({ "extension": "every", "values": { "greeting": "tok-4f9e2a", "every": 5 } }),
        ))
        .await
        .unwrap();
    let requested = stage.harness.trail(saved.request)?[0]
        .details()
        .value()
        .clone();

    assert_eq!(
        requested["params"],
        json!({ "extension": "every", "keys": ["every", "greeting"] })
    );
    assert!(!stage.journal_text()?.contains("tok-4f9e2a"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn values_outside_the_schema_are_refused() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let refused = stage
        .try_call(
            "extension.settings.set",
            json!({ "extension": "every", "values": { "every": "often" } }),
        )
        .await;

    assert!(
        rejection(refused)
            .message
            .contains("must be of type integer")
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_cannot_change_settings() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let agent = stage.harness.agent("helper").await?;
    let call = ActionCall::new("extension.settings.set")
        .params(json!({ "extension": "every", "values": {} }));

    assert_eq!(rejection(agent.call(call).await).code, ErrorCode::DENIED);

    Ok(())
}
