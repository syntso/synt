use serde_json::json;
use synt_proto::{ActionCall, ErrorCode};

use super::Stage;
use crate::common::rejection;

#[tokio::test(flavor = "multi_thread")]
async fn the_person_fills_the_settings_form_and_the_extension_reads_it() {
    let stage = Stage::start().await;

    let form = stage
        .call("extension.settings.get", json!({ "extension": "every" }))
        .await;
    stage
        .call(
            "extension.settings.set",
            json!({ "extension": "every", "values": { "greeting": "hi", "every": 5 } }),
        )
        .await;

    assert_eq!(form["schema"]["properties"]["every"]["type"], "integer");
    assert_eq!(form["values"], json!({}));
    assert_eq!(
        stage.every("settings", json!({})).await,
        json!({ "greeting": "hi", "every": 5 })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn values_outside_the_schema_are_refused() {
    let stage = Stage::start().await;

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
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_cannot_change_settings() {
    let stage = Stage::start().await;
    let agent = stage.harness.agent("helper").await;
    let call = ActionCall::new("extension.settings.set")
        .params(json!({ "extension": "every", "values": {} }));

    assert_eq!(rejection(agent.call(call).await).code, ErrorCode::DENIED);
}
