use std::time::Duration;

use synt_platform::{Pty, PtyOutput};
use tokio::time;

const LIMIT: Duration = Duration::from_secs(20);

async fn drain(output: &mut PtyOutput) -> Vec<u8> {
    let mut shown = Vec::new();

    while let Some(data) = output.next().await {
        shown.extend(data);
    }

    shown
}

#[tokio::test(flavor = "multi_thread")]
async fn the_login_shell_ends_with_its_exit_status() {
    let (pty, mut output) = Pty::open("xterm-256color", 80, 24).unwrap();

    pty.write(b"exit 3\r".to_vec()).await.unwrap();
    let shown = time::timeout(LIMIT, drain(&mut output)).await.unwrap();

    assert!(!shown.is_empty());
    assert_eq!(output.end().exit, Some(3));
    assert!(!pty.program().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_resized_shell_ends_when_it_is_hung_up() {
    let (pty, mut output) = Pty::open("xterm-256color", 80, 24).unwrap();

    pty.resize(120, 40).unwrap();
    pty.hang_up().unwrap();
    time::timeout(LIMIT, drain(&mut output)).await.unwrap();

    let end = output.end();
    assert!(end.exit.is_some() || end.signal.is_some(), "{end:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stopped_shell_ends() {
    let (pty, mut output) = Pty::open("xterm-256color", 80, 24).unwrap();

    pty.stop();
    time::timeout(LIMIT, drain(&mut output)).await.unwrap();

    assert!(pty.resize(100, 30).is_err());
}
