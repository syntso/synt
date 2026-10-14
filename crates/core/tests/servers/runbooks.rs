use std::time::Duration;

use bytes::Bytes;
use serde_json::{Value, json};
use synt_proto::{ActionCall, ActionOutcome, StreamId};
use tokio::time;

use crate::Lab;
use crate::lab::{INNER, PROJECT};

const LIMIT: Duration = Duration::from_secs(20);
const RELEASE: &str = "/tmp/synt-runbook-release";

const MARKED: &str = "printf '\\033]133;A\\007$ \\033]133;B\\007'; \
                      printf '\\033]133;C;cmdline=echo inside\\007'; echo inside; \
                      printf '\\033]133;D;0\\007'; \
                      printf '\\033]133;A\\007$ \\033]133;B\\007'; \
                      printf '\\033]133;C;cmdline=hostname\\007'; hostname; \
                      printf '\\033]133;D;0\\007'\n";

fn releasing() -> Value {
    json!({
        "name": "Release",
        "params": [
            { "name": "host", "kind": "target", "default": INNER },
            { "name": "folder", "kind": "path", "default": RELEASE },
        ],
        "steps": [
            {
                "title": "Create the release folder",
                "action": "ssh.exec",
                "target": "{host}",
                "params": { "command": "mkdir -p {folder} && touch {folder}/started" },
            },
            {
                "title": "Check that the release is ready",
                "action": "ssh.exec",
                "target": "{host}",
                "params": { "command": "test -f {folder}/ready" },
                "retry": { "attempts": 2, "every_seconds": 1 },
                "on_failure": "rollback",
            },
            {
                "title": "Never reached",
                "action": "ssh.exec",
                "target": "{host}",
                "params": { "command": "touch {folder}/never" },
            },
        ],
        "rollback": [{
            "title": "Remove the release folder",
            "action": "ssh.exec",
            "target": "{host}",
            "params": { "command": "rm -rf {folder}" },
        }],
    })
}

async fn put(lab: &Lab, name: &str, runbook: &Value) {
    let call = ActionCall::new("runbook.put").params(json!({
        "project": PROJECT,
        "name": name,
        "runbook": runbook,
    }));

    lab.client.call(call).await.unwrap();
}

async fn run(lab: &Lab, name: &str) -> ActionOutcome {
    let call = ActionCall::new("runbook.run").params(json!({
        "project": PROJECT,
        "name": name,
        "approve_steps": true,
    }));

    lab.client.call(call).await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_runbook_whose_check_fails_removes_what_it_made() {
    let Some(lab) = Lab::start().await else {
        return;
    };
    put(&lab, "release", &releasing()).await;

    let outcome = run(&lab, "release").await;
    let report = &outcome.value;
    let left = lab
        .exec(INNER, &format!("test -e {RELEASE}"))
        .await
        .unwrap();

    assert_eq!(report["state"], json!("rolled_back"));
    assert_eq!(report["steps"][0]["state"], json!("succeeded"));
    assert_eq!(report["steps"][1]["state"], json!("failed"));
    assert_eq!(report["steps"][1]["attempts"], json!(2));
    assert_eq!(report["steps"][1]["error"], json!("exit 1, expected 0"));
    assert_eq!(report["steps"][2]["state"], json!("skipped"));
    assert_eq!(report["rollback"][0]["state"], json!("succeeded"));
    assert_eq!(left.value["exit"], json!(1));

    let finished = lab.finished(outcome.request).await;
    assert_eq!(finished["state"], json!("rolled_back"));
    assert!(!finished.to_string().contains("stdout"));

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn commands_typed_in_a_terminal_become_a_runbook_that_runs_on_the_same_host() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let opened = lab
        .call(
            INNER,
            "ssh.session.open",
            json!({ "cols": 100, "rows": 30 }),
        )
        .await
        .unwrap();
    let stream: StreamId = serde_json::from_value(opened.value["stream"].clone()).unwrap();
    let session = opened.value["session"].clone();
    let _output = lab.client.stream(stream);

    lab.client
        .send_stream(stream, Bytes::from_static(MARKED.as_bytes()))
        .await
        .unwrap();
    time::timeout(LIMIT, two_blocks(&lab, &session))
        .await
        .unwrap();

    let drafting = ActionCall::new("runbook.from_blocks").params(json!({
        "session": session,
        "blocks": [1, 2],
        "name": "Look around",
    }));
    let drafted = lab.client.call(drafting).await.unwrap();
    let runbook = drafted.value["runbook"].clone();

    assert_eq!(
        runbook["params"],
        json!([{ "name": "host", "kind": "target", "default": INNER }])
    );
    assert_eq!(
        runbook["steps"][0],
        json!({
            "title": "echo inside",
            "action": "ssh.exec",
            "target": "{host}",
            "params": { "command": "echo inside" },
        })
    );
    assert_eq!(
        runbook["steps"][1]["params"],
        json!({ "command": "hostname" })
    );

    put(&lab, "look-around", &runbook).await;
    let outcome = run(&lab, "look-around").await;

    assert_eq!(outcome.value["state"], json!("completed"));
    assert_eq!(
        outcome.value["steps"][0]["value"]["stdout"],
        json!("inside\n")
    );

    lab.stop().await;
}

async fn two_blocks(lab: &Lab, session: &Value) {
    loop {
        let call = ActionCall::new("session.blocks").params(json!({ "session": session }));
        let listed = lab.client.call(call).await.unwrap();

        if listed.value.as_array().unwrap().len() >= 2 {
            return;
        }

        time::sleep(Duration::from_millis(50)).await;
    }
}
