#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{PROJECT_ID, SyncingCore, eventually, rejection};
use serde_json::{Value, json};
use synt_journal::EventKind;
use synt_proto::{ActionCall, ErrorCode, RiskLevel};
use tempfile::TempDir;

const SOON: Duration = Duration::from_millis(100);

struct Pair {
    shared: TempDir,
    alice: SyncingCore,
    bob: SyncingCore,
}

impl Pair {
    async fn start(every: Option<Duration>) -> Self {
        let shared = tempfile::tempdir().unwrap();
        let alice = SyncingCore::start(shared.path(), every);
        let bob = SyncingCore::start(shared.path(), every);

        alice.open_and_enable().await;
        bob.open_and_enable().await;
        alice.share_with(&bob).await;

        let pair = Self { shared, alice, bob };
        pair.settle().await;

        pair
    }

    async fn settle(&self) {
        self.alice.sync().await;
        self.bob.sync().await;
        self.alice.sync().await;
    }
}

fn files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();

        if path.is_dir() {
            found.extend(files(&path));
        } else {
            found.push(path);
        }
    }

    found
}

async fn status(core: &SyncingCore) -> Value {
    core.call("sync.status", json!({ "project": "orders" }))
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn two_cores_keep_a_project_in_step_both_ways() {
    let pair = Pair::start(Some(SOON)).await;

    pair.alice.put_host("api-1", "10.0.1.12", "deploy").await;
    let seen = eventually("bob sees api-1", async || pair.bob.host("api-1").await).await;
    assert_eq!(seen["address"], "10.0.1.12");

    pair.bob.put_host("api-1", "10.0.9.9", "deploy").await;
    eventually("alice sees the move", async || {
        let host = pair.alice.host("api-1").await?;
        (host["address"] == "10.0.9.9").then_some(())
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn edits_made_at_the_same_time_merge() {
    let pair = Pair::start(None).await;
    pair.alice.put_host("api-1", "10.0.1.12", "deploy").await;
    pair.settle().await;

    pair.alice.put_host("api-1", "10.0.9.9", "deploy").await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    pair.bob.put_host("api-1", "10.0.1.12", "ops").await;
    pair.settle().await;
    pair.bob.sync().await;

    for core in [&pair.alice, &pair.bob] {
        let text = fs::read_to_string(core.project().join("resources/api-1.toml")).unwrap();
        let merged: toml::Table = text.parse().unwrap();

        assert_eq!(merged["address"].as_str(), Some("10.0.9.9"));
        assert_eq!(merged["user"].as_str(), Some("ops"));
        assert_eq!(status(core).await["conflicts"], json!([]));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conflict_is_shown_and_resolved_through_the_actions() {
    let pair = Pair::start(None).await;
    pair.alice.put_host("api-1", "10.0.1.12", "deploy").await;
    pair.settle().await;

    pair.alice.put_host("api-1", "10.0.7.7", "deploy").await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    pair.bob.put_host("api-1", "10.0.8.8", "deploy").await;
    pair.settle().await;

    let conflict = status(&pair.alice).await["conflicts"][0].clone();
    assert_eq!(conflict["path"], "resources/api-1.toml");
    assert_eq!(conflict["fields"], json!(["address"]));

    let alice = pair.alice.device().await;
    let history = pair
        .alice
        .call(
            "document.history",
            json!({ "project": "orders", "path": "resources/api-1.toml" }),
        )
        .await;
    let mine = history
        .as_array()
        .unwrap()
        .iter()
        .find(|revision| {
            revision["author"] == alice.as_str()
                && conflict["candidates"]
                    .as_array()
                    .unwrap()
                    .contains(&revision["revision"])
        })
        .unwrap();

    pair.alice
        .call(
            "document.resolve",
            json!({
                "project": "orders",
                "path": "resources/api-1.toml",
                "revision": mine["revision"],
            }),
        )
        .await;
    pair.settle().await;
    pair.bob.sync().await;

    for core in [&pair.alice, &pair.bob] {
        let host = eventually("the resolution is loaded", async || {
            core.host("api-1")
                .await
                .filter(|host| host["address"] == "10.0.7.7")
        })
        .await;

        assert_eq!(host["address"], "10.0.7.7");
        assert_eq!(status(core).await["conflicts"], json!([]));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_a_member_rotates_the_project_key() {
    let pair = Pair::start(None).await;
    let carol = SyncingCore::start(pair.shared.path(), None);
    carol.open_and_enable().await;
    pair.alice.share_with(&carol).await;
    carol.sync().await;
    let removed = carol.device().await;

    let change = pair
        .alice
        .call(
            "sync.unshare",
            json!({ "project": "orders", "member": removed }),
        )
        .await;
    pair.alice.put_host("db", "10.0.2.2", "postgres").await;
    pair.settle().await;
    pair.bob.sync().await;
    carol.sync().await;

    assert_eq!(change["generation"], 2);
    assert_eq!(status(&pair.bob).await["generation"], 2);
    assert!(pair.bob.project().join("resources/db.toml").exists());
    assert_eq!(status(&carol).await["access"], "removed");
    assert!(!carol.project().join("resources/db.toml").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_shared_folder_never_shows_a_host_name() {
    let pair = Pair::start(None).await;
    pair.alice
        .put_host("primary-db", "db-primary.internal.example", "postgres")
        .await;
    pair.settle().await;

    let stored = files(pair.shared.path());
    assert!(!stored.is_empty());

    for file in stored {
        let text = String::from_utf8_lossy(&fs::read(&file).unwrap()).into_owned();
        let name = file.to_string_lossy().into_owned();

        for word in [
            "db-primary",
            "internal.example",
            "primary-db",
            "postgres",
            "orders",
        ] {
            assert!(!text.contains(word), "{} shows {word}", file.display());
            assert!(
                !name.contains(word),
                "{} is named after {word}",
                file.display()
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_actions_are_described_with_their_risk_and_owner() {
    let shared = tempfile::tempdir().unwrap();
    let core = SyncingCore::start(shared.path(), None);
    let app = core.harness().app().await;

    let described = app.describe().await.unwrap();
    let rule = |id: &str| {
        let description = described.iter().find(|action| action.id == id).unwrap();

        (description.risk, description.person_only)
    };

    assert_eq!(rule("sync.enable"), (RiskLevel::Write, true));
    assert_eq!(rule("sync.disable"), (RiskLevel::Write, true));
    assert_eq!(rule("sync.status"), (RiskLevel::Read, false));
    assert_eq!(rule("sync.share"), (RiskLevel::Write, true));
    assert_eq!(rule("sync.unshare"), (RiskLevel::Write, true));
    assert_eq!(rule("document.history"), (RiskLevel::Read, false));
    assert_eq!(rule("document.resolve"), (RiskLevel::Write, true));

    let enable = described
        .iter()
        .find(|action| action.id == "sync.enable")
        .unwrap();
    assert!(enable.params["properties"]["folder"].is_object());
    assert!(enable.params["properties"]["project"].is_object());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_may_look_but_not_share() {
    let pair = Pair::start(None).await;
    let agent = pair.alice.harness().agent("deploy-bot").await;

    let looked = agent
        .call(ActionCall::new("sync.status").params(json!({ "project": "orders" })))
        .await
        .unwrap();
    let shared = rejection(
        agent
            .call(
                ActionCall::new("sync.share")
                    .params(json!({ "project": "orders", "member": "ab".repeat(16) })),
            )
            .await,
    );

    assert_eq!(looked.value["access"], "member");
    assert_eq!(shared.code, ErrorCode::DENIED);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_journal_records_calls_and_applied_changes_without_contents() {
    let pair = Pair::start(None).await;
    pair.alice
        .put_host("primary-db", "db-primary.internal.example", "postgres")
        .await;
    pair.settle().await;

    let events = pair.bob.harness().events();
    let applied: Vec<_> = events
        .iter()
        .filter(|event| event.kind() == EventKind::Note && event.action().as_str() == "sync.apply")
        .collect();
    let enabled = events
        .iter()
        .find(|event| {
            event.kind() == EventKind::Finished && event.action().as_str() == "sync.enable"
        })
        .unwrap();

    assert!(
        applied
            .iter()
            .any(|event| event.details().value()["path"] == "resources/primary-db.toml")
    );
    assert_eq!(enabled.details().value()["project"], PROJECT_ID);
    for event in &events {
        let text = event.details().value().to_string();

        assert!(!text.contains("db-primary"), "{text}");
    }
    assert!(pair.bob.harness().is_intact());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_project_key_and_device_key_live_in_the_vault() {
    let pair = Pair::start(None).await;

    for core in [&pair.alice, &pair.bob] {
        let items = core.vault_items();

        assert!(items.contains(&"sync.device".to_owned()));
        assert!(items.contains(&format!("sync.project.{PROJECT_ID}")));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_closed_or_unknown_project_cannot_sync() {
    let shared = tempfile::tempdir().unwrap();
    let core = SyncingCore::start(shared.path(), None);
    let app = core.harness().app().await;

    let unknown = rejection(
        app.call(
            ActionCall::new("sync.enable")
                .params(json!({ "project": "nowhere", "folder": shared.path() })),
        )
        .await,
    );
    let relative = rejection(
        app.call(
            ActionCall::new("sync.enable").params(json!({ "project": "orders", "folder": "x" })),
        )
        .await,
    );
    let personal = rejection(
        app.call(
            ActionCall::new("sync.enable")
                .params(json!({ "project": "personal", "folder": shared.path() })),
        )
        .await,
    );

    assert_eq!(unknown.code, ErrorCode::ACTION_FAILED);
    assert_eq!(relative.code, ErrorCode::INVALID_PARAMS);
    assert!(
        personal.message.contains("does not sync"),
        "{}",
        personal.message
    );
}
