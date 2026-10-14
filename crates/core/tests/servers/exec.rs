use serde_json::json;
use synt_journal::EventKind;

use crate::lab::{BASTION, INNER};
use crate::{Lab, refusal};

const ONE_MEBIBYTE: usize = 1024 * 1024;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_command_is_answered_and_journaled_with_its_exit_and_output_sizes() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let outcome = lab
        .exec(BASTION, "echo out; echo err >&2; exit 3")
        .await
        .unwrap();
    let finished = lab.finished(outcome.request).await;

    assert_eq!(outcome.value["exit"], json!(3));
    assert_eq!(outcome.value["stdout"], json!("out\n"));
    assert_eq!(outcome.value["stderr"], json!("err\n"));
    assert_eq!(
        lab.kinds(outcome.request),
        [
            EventKind::Requested,
            EventKind::Started,
            EventKind::Finished
        ]
    );
    assert_eq!(finished["exit"], json!(3));
    assert_eq!(finished["stdout_bytes"], json!(4));
    assert_eq!(finished["stderr_bytes"], json!(4));
    assert_eq!(finished["truncated"], json!(false));
    assert!(finished["kex"].is_string());
    assert!(finished.get("stdout").is_none());

    let requested = &lab.trail(outcome.request)[0];
    assert_eq!(requested.resource().unwrap().as_str(), BASTION);

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn output_past_a_mebibyte_is_cut_and_journaled_as_truncated() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let outcome = lab.exec(BASTION, "yes a | head -c 3000000").await.unwrap();
    let finished = lab.finished(outcome.request).await;

    assert_eq!(
        outcome.value["stdout"].as_str().unwrap().len(),
        ONE_MEBIBYTE
    );
    assert_eq!(outcome.value["truncated"], json!(true));
    assert_eq!(finished["stdout_bytes"], json!(ONE_MEBIBYTE));
    assert_eq!(finished["truncated"], json!(true));

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_command_past_its_timeout_fails_is_killed_and_is_journaled() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let refused = refusal(
        lab.call(
            BASTION,
            "ssh.exec",
            json!({ "command": "sleep 31340", "timeout_seconds": 1 }),
        )
        .await,
    );
    let journaled = lab.last_failure("ssh.exec");
    let left = lab
        .exec(BASTION, "pgrep -x -f 'sleep 31340'")
        .await
        .unwrap();

    assert!(refused.message.contains("did not finish within 1 s"));
    assert_eq!(journaled["error"], json!(refused.message));
    assert_eq!(left.value["exit"], json!(1));
    assert_eq!(left.value["stdout"], json!(""));

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_host_with_a_via_is_reached_through_the_bastion() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let direct = lab.exec(BASTION, "hostname").await.unwrap();
    let jumped = lab.exec(INNER, "hostname").await.unwrap();
    let connections = lab.call(BASTION, "ssh.connections", json!({})).await;
    let finished = lab.finished(jumped.request).await;

    assert_eq!(jumped.value["exit"], json!(0));
    assert_ne!(jumped.value["stdout"], direct.value["stdout"]);
    assert_eq!(finished["exit"], json!(0));

    let listed = connections.unwrap().value;
    let inner = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|connection| connection["target"] == json!("synt@target"))
        .unwrap();
    assert_eq!(inner["state"], json!("connected"));
    let bastion = format!("synt@{}", lab.servers.bastion_address());
    assert_eq!(inner["via"], json!(bastion));

    lab.stop().await;
}
