use serde_json::{Value, json};
use tempfile::TempDir;

use super::Stage;
use super::stage::PROJECT;

const NOTE: &str = "notes/plan.note";

struct Devices {
    _shared: TempDir,
    alice: Stage,
    bob: Stage,
}

impl Devices {
    async fn start() -> Self {
        let shared = tempfile::tempdir().unwrap();
        let alice = Stage::start().await;
        let bob = Stage::start().await;

        for stage in [&alice, &bob] {
            stage
                .call(
                    "sync.enable",
                    json!({ "project": PROJECT, "folder": shared.path() }),
                )
                .await;
        }

        let status = bob.call("sync.status", json!({ "project": PROJECT })).await;
        alice
            .call(
                "sync.share",
                json!({ "project": PROJECT, "member": status["device"] }),
            )
            .await;

        let devices = Self {
            _shared: shared,
            alice,
            bob,
        };
        devices.settle().await;

        devices
    }

    async fn settle(&self) {
        self.alice.sync().await;
        self.bob.sync().await;
        self.alice.sync().await;
    }
}

async fn write(stage: &Stage, text: &str, base: Option<&str>) -> Value {
    stage
        .every(
            "write",
            json!({ "project": PROJECT, "path": NOTE, "text": text, "base": base }),
        )
        .await
}

async fn read(stage: &Stage) -> Value {
    stage
        .every("read", json!({ "project": PROJECT, "path": NOTE }))
        .await
}

fn revision(document: &Value) -> String {
    document["info"]["revision"].as_str().unwrap().to_owned()
}

fn lines(document: &Value) -> Vec<String> {
    let mut lines: Vec<String> = document["text"]
        .as_str()
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();

    lines
}

#[tokio::test(flavor = "multi_thread")]
async fn a_note_written_on_one_device_reaches_the_other() {
    let devices = Devices::start().await;

    write(&devices.alice, "one", None).await;
    devices.settle().await;

    assert_eq!(read(&devices.bob).await["text"], "one");
}

#[tokio::test(flavor = "multi_thread")]
async fn notes_changed_on_both_devices_are_merged_by_the_extension() {
    let devices = Devices::start().await;
    write(&devices.alice, "one", None).await;
    devices.settle().await;

    let alice_base = revision(&read(&devices.alice).await);
    let bob_base = revision(&read(&devices.bob).await);
    write(&devices.alice, "one\ntwo", Some(&alice_base)).await;
    write(&devices.bob, "one\nthree", Some(&bob_base)).await;
    devices.settle().await;
    devices.settle().await;

    let on_alice = read(&devices.alice).await;
    let on_bob = read(&devices.bob).await;

    assert_eq!(lines(&on_alice), ["one", "three", "two"]);
    assert_eq!(on_alice["text"], on_bob["text"]);
}
