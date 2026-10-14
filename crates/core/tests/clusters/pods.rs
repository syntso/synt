use std::time::Duration;

use serde_json::{Value, json};
use synt_proto::RiskLevel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time;

use crate::Lab;
use crate::lab::{DIRECT, INSIDE, ticker};

const PATIENCE: Duration = Duration::from_secs(180);

pub async fn running_pod(lab: &Lab, app: &str) -> String {
    let params = json!({ "kind": "pods", "selector": format!("app={app}") });

    time::timeout(PATIENCE, async {
        loop {
            let listed = lab
                .person(DIRECT, "k8s.list", params.clone())
                .await
                .unwrap();
            let running = listed.value["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|pod| {
                    pod["status"]["phase"] == "Running"
                        && pod["metadata"]["deletionTimestamp"].is_null()
                });

            if let Some(pod) = running {
                return pod["metadata"]["name"].as_str().unwrap().to_owned();
            }

            time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no pod of {app} ran within {PATIENCE:?}"))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_command_in_a_pod_is_asked_like_ssh_exec_and_its_output_is_not_journaled() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.person(DIRECT, "k8s.apply", json!({ "manifest": ticker("worker") }))
        .await
        .unwrap();
    let pod = running_pod(&lab, "worker").await;

    let (plain_question, plain) = lab
        .agent_asking(
            INSIDE,
            "k8s.exec",
            json!({ "pod": pod, "command": ["cat", "/www/index.html"] }),
        )
        .await;
    let (shell_question, shell) = lab
        .agent_asking(
            DIRECT,
            "k8s.exec",
            json!({ "pod": pod, "command": ["sh", "-c", "exit 3"] }),
        )
        .await;

    let plain = plain.unwrap();
    let shell = shell.unwrap();
    let journaled = lab.finished(plain.request).await;

    assert_eq!(plain_question.risk, RiskLevel::Write);
    assert_eq!(shell_question.risk, RiskLevel::Privileged);
    assert_eq!(plain.value["stdout"], "hello from worker\n");
    assert_eq!(shell.value["exit"], 3);
    assert_eq!(journaled["exit"], 0);
    assert_eq!(journaled["stdout_bytes"], 18);
    assert!(!lab.journal_text().contains("hello from worker"));

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn logs_are_returned_to_the_caller_and_only_counted_in_the_journal() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.person(DIRECT, "k8s.apply", json!({ "manifest": ticker("talker") }))
        .await
        .unwrap();
    let pod = running_pod(&lab, "talker").await;
    time::sleep(Duration::from_secs(2)).await;

    let logs = lab
        .agent(INSIDE, "k8s.logs", json!({ "pod": pod, "tail": 2 }))
        .await
        .unwrap();
    let journaled = lab.finished(logs.request).await;
    let text = logs.value["text"].as_str().unwrap();

    assert_eq!(logs.value["lines"], 2);
    assert!(text.starts_with("tick "), "{text}");
    assert_eq!(
        journaled,
        json!({ "bytes": text.len(), "lines": 2, "truncated": false })
    );
    assert!(!lab.journal_text().contains(text.lines().next().unwrap()));

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_port_forward_is_held_until_it_is_closed() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.person(DIRECT, "k8s.apply", json!({ "manifest": ticker("web") }))
        .await
        .unwrap();
    running_pod(&lab, "web").await;

    let (question, opened) = lab
        .agent_asking(
            INSIDE,
            "k8s.port_forward",
            json!({ "service": "web", "port": 80 }),
        )
        .await;
    let opened = opened.unwrap();
    let port = u16::try_from(opened.value["local_port"].as_u64().unwrap()).unwrap();

    let page = get(port).await;
    let listed = lab
        .agent("", "k8s.port_forward.list", json!({}))
        .await
        .unwrap();
    let id = opened.value["id"].clone();
    let (_, closed) = lab
        .agent_asking("", "k8s.port_forward.close", json!({ "id": id }))
        .await;
    let closed = closed.unwrap();
    let after = TcpStream::connect(("127.0.0.1", port)).await;

    assert_eq!(question.risk, RiskLevel::Write);
    assert!(page.contains("hello from web"), "{page}");
    assert_eq!(listed.value.as_array().unwrap().len(), 1);
    assert_eq!(listed.value[0]["remote_port"], 8080);
    assert!(after.is_err(), "the local port is closed with the forward");
    assert_eq!(
        lab.finished(opened.request).await["local_port"],
        u64::from(port)
    );
    let closing: Value = lab.finished(closed.request).await;
    assert_eq!(closing["connections"], 1);
    assert!(closing["received_bytes"].as_u64().unwrap() > 0);
    assert!(!lab.journal_text().contains("hello from web"));

    lab.finish().await;
}

async fn get(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream
        .write_all(b"GET /index.html HTTP/1.0\r\nHost: web\r\n\r\n")
        .await
        .unwrap();

    let mut answer = String::new();
    time::timeout(Duration::from_secs(30), stream.read_to_string(&mut answer))
        .await
        .unwrap()
        .unwrap();

    answer
}
