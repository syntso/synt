use std::time::Duration;

use serde_json::json;
use tokio::time;

use crate::lab::BASTION;
use crate::{Lab, refusal};

const GRACE: Duration = Duration::from_millis(300);

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn calls_to_one_host_share_a_connection_that_closes_after_the_grace() {
    let Some(lab) = Lab::with_grace(GRACE).await else {
        return;
    };

    let (first, second) = tokio::join!(lab.client_port(BASTION), lab.client_port(BASTION));
    let held = lab
        .call(BASTION, "ssh.connections", json!({}))
        .await
        .unwrap();
    time::sleep(GRACE * 4).await;
    let after_grace = lab
        .call(BASTION, "ssh.connections", json!({}))
        .await
        .unwrap();
    let third = lab.client_port(BASTION).await;

    assert_eq!(first, second);
    assert_eq!(held.value.as_array().unwrap().len(), 1);
    assert_eq!(held.value[0]["state"], json!("connected"));
    assert_eq!(after_grace.value, json!([]));
    assert_ne!(first, third);

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_dropped_connection_fails_its_call_and_the_next_call_reconnects() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let first = lab.client_port(BASTION).await;
    let dropped = refusal(lab.exec(BASTION, "kill -KILL $PPID; sleep 5").await);
    let journaled = lab.last_failure("ssh.exec");
    let asked = lab.last_approval("ssh.exec");
    let second = lab.client_port(BASTION).await;

    assert!(dropped.message.contains("connection was lost"));
    assert_eq!(journaled["error"], json!(dropped.message));
    assert_ne!(first, second);
    assert_eq!(asked["scope"], json!("once"));

    lab.stop().await;
}
