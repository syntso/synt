use std::num::NonZeroU16;
use std::time::Duration;

use synt_kube::{
    Command, ExecRequest, ForwardRequest, ForwardTarget, KubeError, LogRequest, ObjectName,
};
use synt_proto::ExecLimits;
use synt_testkit::TestError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time;

use crate::Lab;
use crate::lab::ticker;

fn command(words: &[&str]) -> Result<Command, KubeError> {
    let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();

    Command::try_from(words)
}

fn port(number: u16) -> Result<NonZeroU16, TestError> {
    Ok(NonZeroU16::new(number).ok_or("a port is never zero")?)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn logs_are_read_whole_or_followed() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    lab.apply(&ticker("talker")).await?;
    let pod = lab.running_pod("app=talker").await?;
    time::sleep(Duration::from_secs(2)).await;

    let request = LogRequest {
        tail_lines: Some(1),
        ..LogRequest::new(pod)
    };
    let tail = request.read(&lab.cluster).await.unwrap();

    let mut followed = request.follow(&lab.cluster).await.unwrap();
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

    lab.finish().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_command_runs_in_a_pod_and_reports_how_it_ended() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    lab.apply(&ticker("worker")).await?;
    let pod = lab.running_pod("app=worker").await?;
    let run = |words: &[&str]| ExecRequest {
        container: Some(ObjectName::try_from("main").unwrap()),
        ..ExecRequest::new(pod.clone(), command(words).unwrap())
    };
    let never = std::future::pending::<()>;
    let cluster = &lab.cluster;

    let greeted = run(&["echo", "hello"])
        .run(cluster, ExecLimits::default(), never())
        .await
        .unwrap();
    let failed = run(&["ls", "/nowhere"])
        .run(cluster, ExecLimits::default(), never())
        .await
        .unwrap();
    let missing = run(&["no-such-program"])
        .run(cluster, ExecLimits::default(), never())
        .await;
    let limits = ExecLimits {
        timeout: Duration::from_secs(1),
        ..ExecLimits::default()
    };
    let slow = run(&["sleep", "30"]).run(cluster, limits, never()).await;

    assert_eq!(greeted.exit, Some(0));
    assert_eq!(greeted.output.stdout, "hello\n");
    assert_ne!(failed.exit, Some(0));
    assert!(failed.exit.is_some());
    assert!(
        failed.output.stderr.contains("nowhere"),
        "{}",
        failed.output.stderr
    );
    assert!(
        missing.is_err()
            || missing
                .as_ref()
                .is_ok_and(|output| output.failure.is_some())
    );
    assert!(slow.is_err());

    lab.finish().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_port_forward_reaches_a_pod_directly_or_through_its_service() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    lab.apply(&ticker("web")).await?;
    let pod = lab.running_pod("app=web").await?;

    let to_pod = ForwardRequest::new(ForwardTarget::Pod(pod.clone()), port(8080)?)
        .open(&lab.cluster)
        .await
        .unwrap();
    let service = ObjectName::try_from("web").unwrap();
    let to_service = ForwardRequest::new(ForwardTarget::Service(service), port(80)?)
        .open(&lab.cluster)
        .await
        .unwrap();

    let from_pod = get(to_pod.local_port()).await?;
    let from_service = get(to_service.local_port()).await?;
    let status = to_service.close().await;

    assert!(from_pod.contains("hello from web"), "{from_pod}");
    assert!(from_service.contains("hello from web"), "{from_service}");
    assert_eq!(status.pod, pod.as_str());
    assert_eq!(status.remote_port, 8080);
    assert_eq!(status.connections, 1);
    assert!(status.received_bytes > 0);

    lab.finish().await?;

    Ok(())
}

async fn get(port: u16) -> Result<String, TestError> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
    stream
        .write_all(b"GET /index.html HTTP/1.0\r\nHost: web\r\n\r\n")
        .await?;

    let mut answer = String::new();
    time::timeout(Duration::from_secs(20), stream.read_to_string(&mut answer)).await??;

    Ok(answer)
}
