#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{Harness, SAMPLE_MANIFEST};
use serde_json::json;
use synt_actions::{ActionError, Risk};

const EVERY_MANIFEST: &str = r#"
id = "every"
name = "Everything"
version = "0.2.0"
api = "0.2"

[capabilities]
call = ["core.ping"]
storage = true
"#;

fn everything() -> Vec<u8> {
    fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/extension/everything.wasm"
    ))
    .unwrap()
}

fn build(harness: &Harness, name: &str, manifest: &str, wasm: &[u8]) -> PathBuf {
    let build = harness.folder.path().join(name);
    fs::create_dir_all(&build).unwrap();

    fs::write(build.join("extension.toml"), manifest).unwrap();
    fs::write(build.join("extension.wasm"), wasm).unwrap();

    build
}

fn failure<T: std::fmt::Debug>(result: Result<T, ActionError>) -> String {
    match result {
        Err(ActionError::Failed(reason)) => reason,
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn api_0_1_and_0_2_extensions_run_side_by_side() {
    let harness = Harness::new();
    harness.install_sample().await;
    let every = build(&harness, "every", EVERY_MANIFEST, &everything());

    let installed = harness.install_local(&every).await.unwrap();
    let echoed = harness.call("t.echo", None, json!({ "old": true })).await;
    let recalled = harness
        .call("every.recall", None, json!({ "key": "nothing" }))
        .await;

    assert_eq!(installed["api"], "0.2");
    assert_eq!(echoed.unwrap(), json!({ "old": true }));
    assert_eq!(recalled.unwrap(), json!({ "value": null }));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_component_must_be_built_for_the_api_its_manifest_names() {
    let harness = Harness::new();
    let newer_manifest = SAMPLE_MANIFEST.replace("api = \"0.1\"", "api = \"0.2\"");
    let older_manifest = EVERY_MANIFEST.replace("api = \"0.2\"", "api = \"0.1\"");

    let sample = build(
        &harness,
        "sample",
        &newer_manifest,
        &common::sample_component(),
    );
    let every = build(&harness, "every", &older_manifest, &everything());

    let sample = failure(harness.install_local(&sample).await);
    let every = failure(harness.install_local(&every).await);

    assert!(sample.contains("not an extension for api 0.2"), "{sample}");
    assert!(every.contains("not an extension for api 0.1"), "{every}");
}

#[tokio::test(flavor = "multi_thread")]
async fn reaching_outside_raises_the_risk_of_every_action() {
    let harness = Harness::new();
    let quiet = build(&harness, "quiet", EVERY_MANIFEST, &everything());
    harness.install_local(&quiet).await.unwrap();
    assert_eq!(
        harness.handle("every.ping").unwrap().describe().risk,
        Risk::Read
    );

    let reaching =
        EVERY_MANIFEST.replace("storage = true", "storage = true\nhttp = [\"example.com\"]");
    let reaching = build(&harness, "reaching", &reaching, &everything());
    harness.install_local(&reaching).await.unwrap();

    assert_eq!(
        harness.handle("every.ping").unwrap().describe().risk,
        Risk::Write
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn contributions_must_name_actions_the_extension_describes() {
    let harness = Harness::new();
    let manifest = EVERY_MANIFEST.to_owned()
        + "\n[[contributes.command]]\ntitle = \"Vanish\"\naction = \"every.vanish\"\n";
    let every = build(&harness, "every", &manifest, &everything());

    let reason = failure(harness.install_local(&every).await);

    assert!(reason.contains("does not describe"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn capabilities_the_manifest_lacks_are_refused_at_run_time() {
    let harness = Harness::new();
    let every = build(&harness, "every", EVERY_MANIFEST, &everything());
    harness.install_local(&every).await.unwrap();

    let fetch = failure(
        harness
            .call(
                "every.fetch",
                None,
                json!({ "url": "https://example.com/" }),
            )
            .await,
    );
    let arm = failure(
        harness
            .call("every.arm", None, json!({ "name": "beat", "seconds": 5 }))
            .await,
    );
    let files = harness.call("every.files", None, json!({})).await.unwrap();

    assert!(fetch.starts_with("not_allowed: "), "{fetch}");
    assert!(arm.starts_with("not_allowed: "), "{arm}");
    assert_eq!(files, json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_view_patch_reaches_the_screen_only_for_a_contributed_view() {
    let harness = Harness::new();
    let manifest = EVERY_MANIFEST.to_owned()
        + "\n[[contributes.sidebar]]\nid = \"overview\"\ntitle = \"Overview\"\nview = \"overview\"\n";
    let every = build(&harness, "every", &manifest, &everything());
    harness.install_local(&every).await.unwrap();

    harness.call("every.patch", None, json!({})).await.unwrap();
    let posted = failure(
        harness
            .call("every.post", None, json!({ "message": "hi" }))
            .await,
    );

    assert_eq!(harness.screen.updates().len(), 1);
    assert!(posted.starts_with("unknown_panel: "), "{posted}");
}
