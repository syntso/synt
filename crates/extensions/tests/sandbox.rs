#![allow(clippy::unwrap_used)]

mod common;

use std::time::Duration;

use common::{Harness, SAMPLE_MANIFEST};
use serde_json::json;
use synt_actions::{ActionError, Context, Risk};
use synt_journal::{Actor, Name, Seq};

fn failure(result: Result<serde_json::Value, ActionError>) -> String {
    match result {
        Err(ActionError::Failed(reason)) => reason,
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn described_actions_join_the_registry() {
    let harness = Harness::new();
    harness.install_sample().await;

    let descriptions = harness.registry.describe();
    let call = descriptions
        .iter()
        .find(|description| description.id.as_str() == "t.call")
        .unwrap();

    assert_eq!(call.title, "Run a command through ssh.exec");
    assert_eq!(call.risk, Risk::Write);
    assert!(call.takes_target);
    assert!(!call.person_only);
    assert_eq!(call.params, json!({ "type": "object" }));

    let ids: Vec<&str> = descriptions
        .iter()
        .map(|description| description.id.as_str())
        .filter(|id| id.starts_with("t."))
        .collect();
    assert_eq!(
        ids,
        ["t.call", "t.echo", "t.fail", "t.grow", "t.spin", "t.store"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn runs_an_action_and_returns_its_json() {
    let harness = Harness::new();
    harness.install_sample().await;

    let output = harness
        .call("t.echo", None, json!({ "name": "Ann" }))
        .await
        .unwrap();

    assert_eq!(output, json!({ "name": "Ann" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn passes_the_extension_error_on() {
    let harness = Harness::new();
    harness.install_sample().await;

    let reason = failure(harness.call("t.fail", None, json!({})).await);

    assert_eq!(reason, "boom: it broke");
}

#[tokio::test(flavor = "multi_thread")]
async fn running_out_of_fuel_stops_only_that_call() {
    let harness = Harness::with_limits(50_000_000, Duration::from_mins(1));
    harness.install_sample().await;

    let reason = failure(harness.call("t.spin", None, json!({})).await);
    assert_eq!(reason, "the extension ran out of fuel");

    let output = harness.call("t.echo", None, json!({ "after": true })).await;
    assert_eq!(output.unwrap(), json!({ "after": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_past_its_time_limit_is_stopped() {
    let harness = Harness::with_limits(u64::MAX, Duration::from_millis(300));
    harness.install_sample().await;

    let reason = failure(harness.call("t.spin", None, json!({})).await);

    assert_eq!(reason, "the extension ran past its time limit of 300ms");
}

#[tokio::test(flavor = "multi_thread")]
async fn memory_is_capped() {
    let harness = Harness::new();
    harness.install_sample().await;

    let reason = failure(harness.call("t.grow", None, json!({})).await);

    assert!(reason.starts_with("the extension failed: "), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn storage_keeps_values_per_extension() {
    let harness = Harness::new();
    harness.install_sample().await;

    let output = harness
        .call("t.store", None, json!({ "count": 3 }))
        .await
        .unwrap();

    assert_eq!(output, json!({ "count": 3 }));
}

#[tokio::test(flavor = "multi_thread")]
async fn storage_needs_the_capability() {
    let harness = Harness::new();
    let manifest = SAMPLE_MANIFEST.replace("storage = true", "storage = false");
    let build = harness.build("no-storage", &manifest);
    harness.install_local(&build).await.unwrap();

    let reason = failure(harness.call("t.store", None, json!({})).await);

    assert!(reason.starts_with("not_allowed: "), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn storage_is_capped_at_one_mebibyte() {
    let harness = Harness::new();
    harness.install_sample().await;

    let large = "x".repeat(1024 * 1024);
    let reason = failure(
        harness
            .call("t.store", None, json!({ "large": large }))
            .await,
    );

    assert!(reason.starts_with("storage_full: "), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_reach_the_core_as_the_extension() {
    let harness = Harness::new();
    harness.install_sample().await;

    let output = harness
        .call("t.call", Some("synt-test"), json!({ "command": "uptime" }))
        .await
        .unwrap();

    assert_eq!(output, json!({ "exit": 0, "stdout": "up 3 days" }));

    let calls = harness.caller.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].extension, Name::new("t").unwrap());
    assert_eq!(calls[0].action, "ssh.exec");
    assert_eq!(calls[0].target.as_deref(), Some("synt-test"));
    assert_eq!(calls[0].params, json!({ "command": "uptime" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_outside_the_capabilities_are_refused() {
    let harness = Harness::new();
    let manifest = SAMPLE_MANIFEST.replace("call = [\"ssh.exec\"]", "call = [\"core.ping\"]");
    let build = harness.build("no-ssh", &manifest);
    harness.install_local(&build).await.unwrap();
    assert_eq!(
        harness.handle("t.echo").unwrap().describe().risk,
        Risk::Read
    );

    let reason = failure(
        harness
            .call("t.call", Some("synt-test"), json!({ "command": "uptime" }))
            .await,
    );

    assert!(reason.starts_with("not_allowed: "), "{reason}");
    assert!(harness.caller.calls().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_action_that_takes_a_target_needs_one() {
    let harness = Harness::new();
    harness.install_sample().await;

    let prepared = harness.prepare("t.call", None, json!({}));

    assert!(matches!(prepared, Err(ActionError::TargetRequired)));
}

const HOG_MANIFEST: &str = r#"
id = "h"
name = "Hog"
version = "0.1.0"
api = "0.1"

[capabilities]
storage = true
"#;

const EXTRA_MODULE: &str = "(core module $Extra (memory 1))";
const FIRST_IMPORT: &str = "(import \"storage-get\"";

const WALL_CLOCK: &str = r#"(import "wasi:clocks/wall-clock@0.2.0" (instance
    (type $datetime (record (field "seconds" u64) (field "nanoseconds" u32)))
    (export "datetime" (type $moment (eq $datetime)))
    (export "now" (func (result $moment)))))"#;

fn hog(harness: &Harness, name: &str, change: impl Fn(String) -> String) -> std::path::PathBuf {
    let text = change(include_str!("fixtures/hog.wat").to_owned());
    let build = harness.folder.path().join(name);

    std::fs::create_dir_all(&build).unwrap();
    std::fs::write(build.join("extension.toml"), HOG_MANIFEST).unwrap();
    std::fs::write(build.join("extension.wasm"), wat::parse_str(&text).unwrap()).unwrap();

    build
}

#[tokio::test(flavor = "multi_thread")]
async fn tables_cannot_grow_past_the_limit() {
    let harness = Harness::new();
    let build = hog(&harness, "tables", |text| text);
    harness.install_local(&build).await.unwrap();

    let reason = failure(harness.call("h.table", None, json!({})).await);

    assert!(reason.starts_with("the extension failed: "), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn extra_memories_are_refused() {
    let harness = Harness::new();
    let extra = "(core instance (instantiate $Extra))\n".repeat(4);
    let build = hog(&harness, "memories", |text| {
        text.replace(EXTRA_MODULE, &format!("{EXTRA_MODULE}\n{extra}"))
    });

    let reason = failure(harness.install_local(&build).await);

    assert!(reason.contains("memory"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn storage_operations_per_call_are_capped() {
    let harness = Harness::new();
    let build = hog(&harness, "storage", |text| text);
    harness.install_local(&build).await.unwrap();

    let reason = failure(harness.call("h.store", None, json!({})).await);

    assert_eq!(
        reason,
        "the extension made more than 1000 storage operations in one call"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flood_of_log_lines_still_finishes() {
    let harness = Harness::new();
    let build = hog(&harness, "logs", |text| text);
    harness.install_local(&build).await.unwrap();

    let output = harness.call("h.log", None, json!({})).await;

    assert_eq!(output.unwrap(), json!({ "type": "object" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn interfaces_outside_the_sandbox_are_not_linked() {
    let harness = Harness::new();
    let build = hog(&harness, "clock", |text| {
        text.replace(FIRST_IMPORT, &format!("{WALL_CLOCK}\n{FIRST_IMPORT}"))
    });

    let reason = failure(harness.install_local(&build).await);

    assert!(reason.contains("wasi:clocks/wall-clock"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_raise_the_risk_of_every_action() {
    let harness = Harness::new();
    harness.install_sample().await;

    let echo = harness.handle("t.echo").unwrap().describe();

    assert_eq!(echo.risk, Risk::Write);
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_to_actions_outside_the_core_are_refused() {
    let harness = Harness::new();
    let manifest = SAMPLE_MANIFEST.replace("call = [\"ssh.exec\"]", "call = [\"other.run\"]");
    let build = harness.build("outside", &manifest);

    let reason = failure(harness.install_local(&build).await);

    assert!(reason.contains("not a core action"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_extension_cannot_run_extension_actions() {
    let harness = Harness::new();
    harness.install_sample().await;

    let prepared = harness.prepare("t.echo", None, json!({})).unwrap();
    let context = Context::new(Actor::Plugin(Name::new("other").unwrap()), Seq::FIRST, None);
    let reason = failure(
        prepared
            .run(&context)
            .await
            .map(|completed| completed.value),
    );

    assert!(
        reason.contains("cannot run the actions of an extension"),
        "{reason}"
    );
}
