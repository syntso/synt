use std::time::Duration;

use synt_kube::{ExecLimits, ExecRequest, ForwardTarget, LogRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time;

use crate::Lab;
use crate::lab::ticker;

fn command(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn logs_are_read_whole_or_followed() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.apply(&ticker("talker")).await;
    let pod = lab.running_pod("app=talker").await;
    time::sleep(Duration::from_secs(2)).await;

    let request = LogRequest {
        pod: pod.clone(),
        tail_lines: Some(1),
        ..LogRequest::default()
    };
    let tail = lab.cluster.logs(&request).await.unwrap();

    let mut followed = lab.cluster.follow_logs(&request).await.unwrap();
    let mut seen = String::new();
    while seen.matches("tick").count() < 3 {
        let chunk = time::timeout(Duration::from_secs(10), followed.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        seen.push_str(&String::from_utf8_lossy(&chunk));
    }

    assert_eq!(tail.lines, 1);
    assert!(tail.text.starts_with("tick "), "{}", tail.text);
    assert!(!tail.truncated);
    assert!(seen.matches("tick").count() >= 3);

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_command_runs_in_a_pod_and_reports_how_it_ended() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.apply(&ticker("worker")).await;
    let pod = lab.running_pod("app=worker").await;
    let run = |words: &[&str]| ExecRequest {
        pod: pod.clone(),
        container: Some("main".to_owned()),
        command: command(words),
        ..ExecRequest::default()
    };
    let never = std::future::pending::<()>;

    let greeted = lab
        .cluster
        .exec(&run(&["echo", "hello"]), ExecLimits::default(), never())
        .await
        .unwrap();
    let failed = lab
        .cluster
        .exec(&run(&["ls", "/nowhere"]), ExecLimits::default(), never())
        .await
        .unwrap();
    let missing = lab
        .cluster
        .exec(&run(&["no-such-program"]), ExecLimits::default(), never())
        .await;
    let limits = ExecLimits {
        timeout: Duration::from_secs(1),
        ..ExecLimits::default()
    };
    let slow = lab
        .cluster
        .exec(&run(&["sleep", "30"]), limits, never())
        .await;

    assert_eq!(greeted.exit, Some(0));
    assert_eq!(greeted.stdout, "hello\n");
    assert_ne!(failed.exit, Some(0));
    assert!(failed.exit.is_some());
    assert!(failed.stderr.contains("nowhere"), "{}", failed.stderr);
    assert!(
        missing.is_err()
            || missing
                .as_ref()
                .is_ok_and(|output| output.failure.is_some())
    );
    assert!(slow.is_err());

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_port_forward_reaches_a_pod_directly_or_through_its_service() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.apply(&ticker("web")).await;
    let pod = lab.running_pod("app=web").await;

    let to_pod = lab
        .cluster
        .forward(&ForwardTarget::Pod(pod.clone()), None, 8080, None)
        .await
        .unwrap();
    let to_service = lab
        .cluster
        .forward(&ForwardTarget::Service("web".to_owned()), None, 80, None)
        .await
        .unwrap();

    let from_pod = get(to_pod.local_port()).await;
    let from_service = get(to_service.local_port()).await;
    let status = to_service.close().await;

    assert!(from_pod.contains("hello from web"), "{from_pod}");
    assert!(from_service.contains("hello from web"), "{from_service}");
    assert_eq!(status.pod, pod);
    assert_eq!(status.remote_port, 8080);
    assert_eq!(status.connections, 1);
    assert!(status.received_bytes > 0);

    lab.finish().await;
}

async fn get(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream
        .write_all(b"GET /index.html HTTP/1.0\r\nHost: web\r\n\r\n")
        .await
        .unwrap();

    let mut answer = String::new();
    time::timeout(Duration::from_secs(20), stream.read_to_string(&mut answer))
        .await
        .unwrap()
        .unwrap();

    answer
}
