#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use common::{FileServer, Harness, SAMPLE_MANIFEST, Signer, everything_component};
use serde_json::{Value, json};
use synt_actions::ActionError;
use synt_extensions::{Remote, Runtime};

const CAPABILITIES: &str = r#"{ "call": ["ssh.exec"], "storage": true }"#;

fn capabilities() -> Value {
    serde_json::from_str(CAPABILITIES).unwrap()
}

fn sample() -> Vec<u8> {
    wat::parse_str(include_str!("fixtures/sample.wat")).unwrap()
}

const SERIAL: u64 = 100;

fn later() -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    now.as_secs() + 3600
}

fn publish(server: &FileServer, signer: &Signer, listing: &Value, serial: u64, expires: u64) {
    let index = json!({ "extensions": [listing] }).to_string().into_bytes();

    server.put(
        "index.json.minisig",
        signer.sign_index(&index, serial, expires).into_bytes(),
    );
    server.put("index.json", index);
}

fn registry_with(signer: &Signer, listing: &Value, wasm: Vec<u8>) -> (FileServer, Harness) {
    let server = FileServer::start();
    publish(&server, signer, listing, SERIAL, later());
    server.put("t-0.1.0.wasm", wasm);

    let remote = Remote::new(server.address(), &signer.public_key()).unwrap();
    (server, Harness::with_remote(remote))
}

fn failure<T>(result: Result<T, ActionError>) -> String {
    match result {
        Err(ActionError::Failed(reason)) => reason,
        Err(other) => panic!("expected a failure, got {other:?}"),
        Ok(_) => panic!("expected a failure, got success"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_build_is_shown_and_recorded_as_unsigned() {
    let harness = Harness::new();
    let build = harness.build("sample", SAMPLE_MANIFEST);
    let params = json!({ "path": build.to_string_lossy() });

    let handle = harness.handle("extension.install_local").unwrap();
    assert_eq!(handle.recorded_params(&params)["signed"], json!(false));

    let prepared = harness
        .prepare("extension.install_local", None, params)
        .unwrap();
    let shown = prepared.shown().unwrap();

    assert_eq!(shown["id"], "t");
    assert_eq!(shown["signed"], json!(false));
    assert_eq!(shown["capabilities"], capabilities());
    assert_eq!(shown["actions"].as_array().unwrap().len(), 6);
    assert!(harness.handle("t.echo").is_none());

    let installed = harness.install_local(&build).await.unwrap();

    assert_eq!(installed["signed"], json!(false));
    assert_eq!(installed["enabled"], json!(true));
    assert!(harness.handle("t.echo").is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn relative_paths_are_refused() {
    let harness = Harness::new();

    let prepared = harness.prepare("extension.install_local", None, json!({ "path": "sample" }));

    assert!(matches!(prepared, Err(ActionError::InvalidParams(_))));
}

#[tokio::test(flavor = "multi_thread")]
async fn ids_of_the_core_are_refused() {
    let harness = Harness::new();
    let manifest = SAMPLE_MANIFEST.replace("id = \"t\"", "id = \"extension\"");
    let build = harness.build("taken", &manifest);

    let reason = failure(harness.install_local(&build).await);

    assert!(reason.contains("taken by the core"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn actions_must_live_under_the_extension_id() {
    let harness = Harness::new();
    let manifest = SAMPLE_MANIFEST.replace("id = \"t\"", "id = \"other\"");
    let build = harness.build("other", &manifest);

    let reason = failure(harness.install_local(&build).await);

    assert!(reason.contains("must be named other.<name>"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn disable_enable_and_remove() {
    let harness = Harness::new();
    harness.install_sample().await;
    harness
        .call("t.store", None, json!({ "kept": 1 }))
        .await
        .unwrap();

    let disabled = harness
        .call("extension.disable", None, json!({ "id": "t" }))
        .await
        .unwrap();
    assert_eq!(disabled["enabled"], json!(false));
    assert!(harness.handle("t.echo").is_none());

    let enabled = harness
        .call("extension.enable", None, json!({ "id": "t" }))
        .await
        .unwrap();
    assert_eq!(enabled["loaded"], json!(true));
    assert!(harness.handle("t.echo").is_some());

    let listed = harness
        .call("extension.list", None, Value::Null)
        .await
        .unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["actions"].as_array().unwrap().len(), 6);

    harness
        .call("extension.remove", None, json!({ "id": "t" }))
        .await
        .unwrap();
    assert!(harness.handle("t.echo").is_none());

    let listed = harness
        .call("extension.list", None, Value::Null)
        .await
        .unwrap();
    assert_eq!(listed, json!([]));
    assert!(!harness.folder.path().join("data/extensions/t").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_extensions_are_reported() {
    let harness = Harness::new();

    let reason = failure(
        harness
            .call("extension.enable", None, json!({ "id": "absent" }))
            .await,
    );

    assert_eq!(reason, "no extension absent is installed");
}

#[tokio::test(flavor = "multi_thread")]
async fn enabled_extensions_load_again_after_a_restart() {
    let harness = Harness::new();
    harness.install_sample().await;
    let Harness { folder, .. } = harness;

    let restarted =
        Harness::in_folder(folder, Runtime::new().unwrap(), Remote::official().unwrap());
    assert_eq!(restarted.extensions.load_enabled().unwrap(), 1);

    let output = restarted
        .call("t.echo", None, json!({ "again": true }))
        .await;
    assert_eq!(output.unwrap(), json!({ "again": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn changed_files_are_not_loaded() {
    let harness = Harness::new();
    harness.install_sample().await;
    let Harness { folder, .. } = harness;

    let installed = folder.path().join("data/extensions/t/extension.wasm");
    let mut bytes = fs::read(&installed).unwrap();
    bytes.extend_from_slice(b"\0\0");
    fs::write(&installed, bytes).unwrap();

    let restarted =
        Harness::in_folder(folder, Runtime::new().unwrap(), Remote::official().unwrap());
    assert_eq!(restarted.extensions.load_enabled().unwrap(), 0);
    assert!(restarted.handle("t.echo").is_none());

    let listed = restarted
        .call("extension.list", None, Value::Null)
        .await
        .unwrap();
    let problem = listed[0]["problem"].as_str().unwrap();
    assert!(
        problem.contains("changed since they were installed"),
        "{problem}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn installs_a_signed_extension_from_the_registry() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (_server, harness) = registry_with(&signer, &listing, sample());

    let found = harness
        .call("extension.search", None, json!({ "query": "host" }))
        .await
        .unwrap();
    assert_eq!(found[0]["id"], "t");
    assert_eq!(found[0]["installed"], Value::Null);

    let prepared = harness
        .prepare("extension.install", None, json!({ "id": "t" }))
        .unwrap();
    let shown = prepared.shown().unwrap();
    assert_eq!(shown["signed"], json!(true));
    assert_eq!(shown["capabilities"], capabilities());

    let installed = harness
        .call("extension.install", None, json!({ "id": "t" }))
        .await
        .unwrap();
    assert_eq!(installed["signed"], json!(true));

    let output = harness
        .call("t.echo", None, json!({ "signed": true }))
        .await;
    assert_eq!(output.unwrap(), json!({ "signed": true }));

    let found = harness
        .call("extension.search", None, Value::Null)
        .await
        .unwrap();
    assert_eq!(found[0]["installed"], "0.1.0");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_signature_from_another_key_is_refused() {
    let signer = Signer::new();
    let stranger = Signer::new();
    let listing = stranger.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (_server, harness) = registry_with(&signer, &listing, sample());

    let reason = failure(harness.prepare("extension.install", None, json!({ "id": "t" })));

    assert!(
        reason.starts_with("the signature does not check out"),
        "{reason}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_download_that_differs_from_the_index_is_refused() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let mut changed = sample();
    changed.extend_from_slice(b"\0");
    let (_server, harness) = registry_with(&signer, &listing, changed);

    let reason = failure(harness.prepare("extension.install", None, json!({ "id": "t" })));

    assert!(reason.contains("the download has sha256"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn capabilities_widened_after_signing_are_refused() {
    let signer = Signer::new();
    let mut listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    listing["capabilities"]["call"] = json!(["ssh.exec", "sftp.write"]);
    let (_server, harness) = registry_with(&signer, &listing, sample());

    let reason = failure(harness.prepare("extension.install", None, json!({ "id": "t" })));

    assert!(reason.contains("differ from the index"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_id_missing_from_the_registry_is_reported() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (_server, harness) = registry_with(&signer, &listing, sample());

    let reason = failure(harness.prepare("extension.install", None, json!({ "id": "absent" })));

    assert_eq!(reason, "the registry has no extension absent");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_older_index_is_refused() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (server, harness) = registry_with(&signer, &listing, sample());

    harness
        .call("extension.search", None, Value::Null)
        .await
        .unwrap();

    publish(&server, &signer, &listing, SERIAL - 1, later());
    let reason = failure(harness.call("extension.search", None, Value::Null).await);

    assert!(reason.contains("older than one already seen"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_expired_index_is_refused() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (server, harness) = registry_with(&signer, &listing, sample());

    publish(&server, &signer, &listing, SERIAL, 1);
    let reason = failure(harness.call("extension.search", None, Value::Null).await);

    assert!(reason.contains("has expired"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_index_signed_by_another_key_is_refused() {
    let signer = Signer::new();
    let stranger = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (server, harness) = registry_with(&signer, &listing, sample());

    publish(&server, &stranger, &listing, SERIAL, later());
    let reason = failure(harness.call("extension.search", None, Value::Null).await);

    assert!(
        reason.starts_with("the signature does not check out"),
        "{reason}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_description_changed_after_signing_is_refused() {
    let signer = Signer::new();
    let mut listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    listing["description"] = json!("Totally harmless.");
    let (_server, harness) = registry_with(&signer, &listing, sample());

    let reason = failure(harness.prepare("extension.install", None, json!({ "id": "t" })));

    assert!(reason.contains("differ from the index"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_older_version_than_the_installed_one_is_refused() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (_server, harness) = registry_with(&signer, &listing, sample());

    let newer = SAMPLE_MANIFEST.replace("version = \"0.1.0\"", "version = \"0.2.0\"");
    let build = harness.build("newer", &newer);
    harness.install_local(&build).await.unwrap();

    let reason = failure(harness.prepare("extension.install", None, json!({ "id": "t" })));

    assert!(
        reason.contains("older than the installed 0.2.0"),
        "{reason}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn replacing_a_signed_extension_with_a_local_build_clears_its_storage() {
    let signer = Signer::new();
    let listing = signer.listing("t", "t-0.1.0.wasm", &sample(), &capabilities());
    let (_server, harness) = registry_with(&signer, &listing, sample());

    harness
        .call("extension.install", None, json!({ "id": "t" }))
        .await
        .unwrap();
    harness
        .call("t.store", None, json!({ "token": "secret" }))
        .await
        .unwrap();

    let build = harness.build("local", SAMPLE_MANIFEST);
    let params = json!({ "path": build.to_string_lossy() });
    let prepared = harness
        .prepare("extension.install_local", None, params)
        .unwrap();
    let replaces = &prepared.shown().unwrap()["replaces"];
    assert_eq!(replaces["signed"], json!(true));
    assert_eq!(replaces["clears_storage"], json!(true));

    harness.install_local(&build).await.unwrap();

    let state = rusqlite::Connection::open(harness.folder.path().join("state.sqlite")).unwrap();
    let items: i64 = state
        .query_row(
            "SELECT COUNT(*) FROM extension_items WHERE extension = 't'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(items, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edited_manifest_is_not_loaded() {
    let harness = Harness::new();
    harness.install_sample().await;
    let Harness { folder, .. } = harness;

    let manifest = folder.path().join("data/extensions/t/extension.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(&manifest, text.replace("storage = true", "storage = false")).unwrap();

    let restarted =
        Harness::in_folder(folder, Runtime::new().unwrap(), Remote::official().unwrap());

    assert_eq!(restarted.extensions.load_enabled().unwrap(), 0);
    assert!(restarted.handle("t.echo").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn installs_a_signed_api_0_2_extension_with_its_contributions() {
    let signer = Signer::new();
    let every = everything_component();
    let capabilities = json!({ "call": ["core.ping"], "storage": true, "timers": true });
    let mut listing = signer.listing_for(
        "0.2",
        "0.2.0",
        "every",
        "every-0.2.0.wasm",
        &every,
        &capabilities,
    );
    listing["contributes"] = json!({
        "command": [{ "title": "Ping the core", "action": "every.ping" }],
        "status": [{ "id": "beat", "title": "Beat", "view": "beat" }]
    });

    let server = FileServer::start();
    publish(&server, &signer, &listing, SERIAL, later());
    server.put("every-0.2.0.wasm", every);
    let harness =
        Harness::with_remote(Remote::new(server.address(), &signer.public_key()).unwrap());

    let shown = harness
        .prepare("extension.install", None, json!({ "id": "every" }))
        .unwrap()
        .shown()
        .cloned()
        .unwrap();
    let installed = harness
        .call("extension.install", None, json!({ "id": "every" }))
        .await
        .unwrap();

    assert_eq!(shown["api"], "0.2");
    assert_eq!(shown["capabilities"], capabilities);
    assert_eq!(shown["contributes"]["status"][0]["view"], "beat");
    assert_eq!(installed["api"], "0.2");
    assert!(harness.handle("every.ping").is_some());
}
