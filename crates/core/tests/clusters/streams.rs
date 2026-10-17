use std::time::Duration;

use serde_json::{Value, json};
use synt_client::Client;
use tokio::io::{self, AsyncBufReadExt, BufReader, DuplexStream, Lines};
use tokio::task::JoinHandle;
use tokio::time;

use crate::Lab;
use crate::lab::{DIRECT, INSIDE, ticker};
use crate::pods::running_pod;

const PATIENCE: Duration = Duration::from_secs(30);

struct Followed {
    lines: Lines<BufReader<DuplexStream>>,
    receiving: JoinHandle<u64>,
}

impl Followed {
    fn open(client: &Client) -> (u32, Self) {
        let mut stream = client.open_stream();
        let id = stream.stream().get();
        let (sink, source) = io::duplex(64 * 1024);
        let client = client.clone();

        let receiving = tokio::spawn(async move {
            let received = client.receive_credited(&mut stream, sink).await.unwrap();
            received.bytes
        });

        let followed = Self {
            lines: BufReader::new(source).lines(),
            receiving,
        };

        (id, followed)
    }

    async fn line(&mut self) -> String {
        let Ok(read) = time::timeout(PATIENCE, self.lines.next_line()).await else {
            panic!("no line in time");
        };
        let Some(line) = read.unwrap() else {
            panic!("the stream closed");
        };
        line
    }

    async fn end(self) -> u64 {
        time::timeout(PATIENCE, self.receiving)
            .await
            .unwrap()
            .unwrap()
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn followed_logs_stream_until_the_caller_closes_them() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.person(DIRECT, "k8s.apply", json!({ "manifest": ticker("chatty") }))
        .await
        .unwrap();
    let pod = running_pod(&lab, "chatty").await;

    let (stream, mut followed) = Followed::open(&lab.agent);
    let started = lab
        .agent(
            INSIDE,
            "k8s.logs",
            json!({ "pod": pod, "follow": true, "stream": stream, "tail": 1 }),
        )
        .await
        .unwrap();

    let first = followed.line().await;
    let second = followed.line().await;
    let closed = lab
        .agent(
            "",
            "k8s.stream.close",
            json!({ "request": started.request.get() }),
        )
        .await
        .unwrap();
    let bytes = followed.end().await;
    let journaled = lab.finished(started.request).await;
    let stranger = lab
        .person(
            "",
            "k8s.stream.close",
            json!({ "request": started.request.get() }),
        )
        .await;

    assert_eq!(
        started.value,
        json!({ "request": started.request.get(), "stream": stream })
    );
    assert!(
        first.starts_with("tick ") && second.starts_with("tick "),
        "{first} {second}"
    );
    assert_eq!(closed.value["request"], started.request.get());
    assert_eq!(journaled["reason"], "closed");
    assert_eq!(journaled["pod"], pod.as_str());
    assert_eq!(journaled["bytes"], bytes);
    assert!(journaled["items"].as_u64().unwrap() >= 2);
    assert!(
        stranger.is_err(),
        "a stream that ended cannot be closed again"
    );
    assert!(!lab.journal_text().contains(&first));

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_watch_streams_one_json_line_per_change() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let (stream, mut followed) = Followed::open(&lab.agent);
    let started = lab
        .agent(
            INSIDE,
            "k8s.watch",
            json!({ "kind": "configmaps", "selector": "watched", "stream": stream }),
        )
        .await
        .unwrap();

    let synced: Value = serde_json::from_str(&followed.line().await).unwrap();
    let manifest = format!(
        "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: seen\n  namespace: {}\n  labels:\n    watched: \"yes\"\n",
        lab.namespace
    );
    lab.person(DIRECT, "k8s.apply", json!({ "manifest": manifest }))
        .await
        .unwrap();
    let added: Value = serde_json::from_str(&followed.line().await).unwrap();

    lab.agent(
        "",
        "k8s.stream.close",
        json!({ "request": started.request.get() }),
    )
    .await
    .unwrap();
    followed.end().await;
    let journaled = lab.finished(started.request).await;

    assert_eq!(synced["type"], "synced");
    assert_eq!(added["type"], "added");
    assert_eq!(added["name"], "seen");
    assert_eq!(added["object"]["metadata"]["labels"]["watched"], "yes");
    assert_eq!(journaled["kind"], "ConfigMap");
    assert_eq!(journaled["items"], 2);
    assert_eq!(journaled["reason"], "closed");

    lab.finish().await;
}
