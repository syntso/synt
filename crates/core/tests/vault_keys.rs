#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use common::{Approver, Grounds, Harness, rejection};
use serde_json::{Value, json};
use synt_actions::Action;
use synt_client::{Client, ClientError};
use synt_core::{Broker, Core, KeyGenerate, KeyImport, KeyList, KeyRemove, SshKeys};
use synt_proto::{ActionCall, ActionOutcome, Actor, ApprovalDecision, ErrorCode, Risk};
use synt_ssh::{Pool, Target};

const PLAIN_KEY: &str = include_str!("fixtures/ed25519");
const PLAIN_PUBLIC: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMZkN7gePpxXwYkMwxMGIoufZFIYx43bfnsPWC5x1DHP";
const PLAIN_FINGERPRINT: &str = "SHA256:pCDmgTzAsIbR/wnxMKQDsMNKXkTh8qukFw/ExMprlK4";
const ENCRYPTED_KEY: &str = include_str!("fixtures/encrypted_ed25519");
const RSA_KEY: &str = include_str!("fixtures/rsa");
const LARGEST_KEY_FILE: usize = 256 * 1024;

struct Keys {
    harness: Harness,
    pool: Pool,
}

fn start() -> Keys {
    let grounds = Grounds::new();
    let keys = SshKeys::new(grounds.vault(), grounds.state());
    let pool = Pool::new();
    let mut registry = grounds.registry();

    registry.register(KeyImport::new(keys.clone())).unwrap();
    registry.register(KeyGenerate::new(keys.clone())).unwrap();
    registry.register(KeyList::new(keys.clone())).unwrap();
    registry
        .register(KeyRemove::new(keys, pool.clone()))
        .unwrap();

    let core = Core::new(registry, grounds.journal(), Broker::new());

    Keys {
        harness: Harness::serve(grounds, core),
        pool,
    }
}

fn generate(name: &str) -> ActionCall {
    ActionCall::new(KeyGenerate::ID).params(json!({ "name": name }))
}

fn import(name: &str, path: &Path) -> ActionCall {
    ActionCall::new(KeyImport::ID).params(json!({ "name": name, "private_key_path": path }))
}

fn remove(name: &str) -> ActionCall {
    ActionCall::new(KeyRemove::ID).params(json!({ "name": name }))
}

fn key_file(harness: &Harness, name: &str, text: &str) -> PathBuf {
    let path = harness.grounds().path().join(name);
    fs::write(&path, text.replace("\r\n", "\n")).unwrap();

    path
}

async fn imported(
    approver: &mut Approver,
    name: &str,
    path: &Path,
) -> Result<ActionOutcome, ClientError> {
    let app = approver.app().clone();
    let (_, outcome) = approver
        .decide(&app, import(name, path), ApprovalDecision::AllowOnce)
        .await;

    outcome
}

async fn refusal(approver: &mut Approver, path: &Path) -> String {
    let error = rejection(imported(approver, "refused", path).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    error.message
}

async fn listed(app: &Client) -> Value {
    app.call(ActionCall::new(KeyList::ID)).await.unwrap().value
}

fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();

    listener.local_addr().unwrap().port()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_generated_key_is_kept_in_the_vault_and_listed_by_its_public_half() {
    let Keys { harness, .. } = start();
    let app = harness.app().await;

    let made = app.call(generate("prod/deploy")).await.unwrap().value;
    let all = listed(&app).await;

    assert_eq!(made["name"], json!("prod/deploy"));
    assert_eq!(made["reference"], json!("secret://prod/deploy"));
    assert_eq!(made["kind"], json!("ssh-ed25519"));
    assert!(
        made["public_key"]
            .as_str()
            .unwrap()
            .starts_with("ssh-ed25519 ")
    );
    assert!(made["fingerprint"].as_str().unwrap().starts_with("SHA256:"));
    assert_eq!(all, json!([made]));
    assert!(
        harness
            .grounds()
            .vault_items()
            .contains(&"ssh-key.prod.deploy".to_owned())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_name_is_taken_once() {
    let Keys { harness, .. } = start();
    let app = harness.app().await;

    app.call(generate("night")).await.unwrap();
    let error = rejection(app.call(generate("night")).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(error.message.contains("already a key named night"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_name_that_could_escape_the_vault_is_refused_before_anything_runs() {
    let Keys { harness, .. } = start();
    let app = harness.app().await;

    for name in ["../night", "a.b", "", "a//b"] {
        let error = rejection(app.call(generate(name)).await);
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS, "{name:?}");
    }
    assert!(harness.grounds().vault_items().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn keys_are_made_imported_and_removed_by_the_person_only() {
    let Keys { harness, .. } = start();
    let agent = harness.agent("deploy-bot").await;
    let path = key_file(&harness, "id_ed25519", PLAIN_KEY);

    for call in [generate("night"), import("night", &path), remove("night")] {
        assert_eq!(rejection(agent.call(call).await).code, ErrorCode::DENIED);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_may_read_the_public_keys() {
    let Keys { harness, .. } = start();
    let app = harness.app().await;
    let agent = harness.agent("deploy-bot").await;

    app.call(generate("night")).await.unwrap();
    let seen = listed(&agent).await;

    assert_eq!(seen, listed(&app).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn importing_is_privileged_even_for_the_person() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let app = approver.app().clone();
    let path = key_file(&harness, "id_ed25519", PLAIN_KEY);

    let (question, outcome) = approver
        .decide(&app, import("night", &path), ApprovalDecision::AllowOnce)
        .await;
    let key = outcome.unwrap().value;

    assert_eq!(question.actor, Actor::Person);
    assert_eq!(question.risk, Risk::Privileged);
    assert_eq!(key["kind"], json!("ssh-ed25519"));
    assert_eq!(key["fingerprint"], json!(PLAIN_FINGERPRINT));
    assert!(
        key["public_key"]
            .as_str()
            .unwrap()
            .starts_with(PLAIN_PUBLIC)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_relative_path_is_refused_before_the_person_is_asked() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;

    let call = import("night", Path::new("id_ed25519"));
    let error = rejection(approver.app().call(call).await);

    assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    assert!(approver.was_not_asked());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_is_not_imported() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;

    let reason = refusal(&mut approver, harness.grounds().path()).await;

    assert_eq!(listed(approver.app()).await, json!([]));
    if cfg!(unix) {
        assert!(reason.contains("not a regular file"), "{reason}");
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_pipe_is_not_imported_and_nothing_waits_for_it() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = harness.grounds().path().join("pipe");
    let made = std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .unwrap();
    assert!(made.success());

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("not a regular file"), "{reason}");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_file_of_another_user_is_not_imported() {
    use std::os::unix::fs::MetadataExt;

    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let foreign = Path::new("/etc/hosts");
    let mine = fs::metadata(harness.grounds().path()).unwrap().uid();
    if fs::metadata(foreign).unwrap().uid() == mine {
        return;
    }

    let reason = refusal(&mut approver, foreign).await;

    assert!(reason.contains("belongs to another user"), "{reason}");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_file_other_users_can_change_is_not_imported() {
    use std::os::unix::fs::PermissionsExt;

    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = key_file(&harness, "id_ed25519", PLAIN_KEY);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("other users can change it"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_too_large_to_be_a_key_is_not_imported() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = key_file(&harness, "huge", &"A".repeat(LARGEST_KEY_FILE + 1));

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("larger than"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_behind_a_passphrase_is_not_imported() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = key_file(&harness, "id_encrypted", ENCRYPTED_KEY);

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("passphrase"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_rsa_key_is_left_to_the_ssh_agent() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = key_file(&harness, "id_rsa", RSA_KEY);

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("RSA"), "{reason}");
    assert!(reason.contains("ssh-agent"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_public_key_is_not_imported() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = key_file(&harness, "id_ed25519.pub", PLAIN_PUBLIC);

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("not a private key"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_file_is_not_imported() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = harness.grounds().path().join("none");

    let reason = refusal(&mut approver, &path).await;

    assert!(reason.contains("no file"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_import_leaves_no_key_behind() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let path = key_file(&harness, "id_rsa", RSA_KEY);

    refusal(&mut approver, &path).await;

    assert_eq!(listed(approver.app()).await, json!([]));
    assert!(harness.grounds().vault_items().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_a_key_forgets_it_and_drops_the_connections_that_use_it() {
    let Keys { harness, pool } = start();
    let app = harness.app().await;
    app.call(generate("night")).await.unwrap();

    let port = closed_port();
    let keyed = Target::new("127.0.0.1", port, "deploy").with_vault_key("night");
    let plain = Target::new("127.0.0.1", port, "plain");
    assert!(pool.acquire(&keyed).await.is_err());
    assert!(pool.acquire(&plain).await.is_err());
    assert_eq!(pool.connections().len(), 2);

    let removed = app.call(remove("night")).await.unwrap().value;
    let left: Vec<String> = pool
        .connections()
        .into_iter()
        .map(|status| status.target)
        .collect();

    assert_eq!(removed["name"], json!("night"));
    assert_eq!(left, [format!("plain@127.0.0.1:{port}")]);
    assert_eq!(listed(&app).await, json!([]));
    assert!(harness.grounds().vault_items().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_a_key_that_does_not_exist_fails() {
    let Keys { harness, .. } = start();
    let app = harness.app().await;

    let error = rejection(app.call(remove("night")).await);

    assert_eq!(error.code, ErrorCode::ACTION_FAILED);
    assert!(error.message.contains("no key named night"));
}

#[tokio::test(flavor = "multi_thread")]
async fn private_keys_never_reach_answers_errors_or_the_journal() {
    let Keys { harness, .. } = start();
    let mut approver = harness.approver().await;
    let plain = key_file(&harness, "id_ed25519", PLAIN_KEY);
    let encrypted = key_file(&harness, "id_encrypted", ENCRYPTED_KEY);
    let rsa = key_file(&harness, "id_rsa", RSA_KEY);
    let secret_text = key_file(&harness, "notes", "password = hunter2-swordfish");

    let mut seen = Vec::new();
    seen.push(approver.app().call(generate("made")).await.unwrap().value);
    seen.push(
        imported(&mut approver, "plain", &plain)
            .await
            .unwrap()
            .value,
    );
    for path in [&encrypted, &rsa, &secret_text] {
        let error = rejection(imported(&mut approver, "refused", path).await);
        seen.push(json!(error.message));
    }
    seen.push(listed(approver.app()).await);
    let journal: Vec<Value> = harness
        .events()
        .iter()
        .map(|event| event.details().value().clone())
        .collect();

    let shown = format!("{seen:?} {journal:?}");
    assert!(!shown.contains("PRIVATE KEY"), "{shown}");
    assert!(!shown.contains("hunter2"), "{shown}");
    for key in [PLAIN_KEY, ENCRYPTED_KEY, RSA_KEY] {
        let body = key.lines().nth(1).unwrap();
        assert!(!shown.contains(body), "{shown}");
    }
}
