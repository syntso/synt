use bytes::Bytes;
use synt_ssh::{Shell, TerminalSize};

use crate::Setup;

const MARKED: &str = "printf '\\033]133;A\\007$ \\033]133;B\\007'; \
                      printf '\\033]133;C\\007'; echo inside; printf '\\033]133;D;5\\007'; exit 7\n";

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_terminal_shell_passes_the_osc_133_marks_and_its_exit_status() {
    let Some(setup) = Setup::start() else { return };
    let lease = setup.pool.acquire(&setup.target()).await.unwrap();
    let size = TerminalSize {
        columns: 120,
        rows: 40,
    };

    let (shell, mut output) = Shell::open(lease, "xterm-256color", size).await.unwrap();
    shell
        .write(Bytes::from_static(MARKED.as_bytes()))
        .await
        .unwrap();

    let mut screen = Vec::new();
    while let Some(data) = output.next().await {
        screen.extend_from_slice(&data);
    }
    let screen = String::from_utf8_lossy(&screen);

    for mark in [
        "\x1b]133;A\x07",
        "\x1b]133;B\x07",
        "\x1b]133;C\x07",
        "\x1b]133;D;5\x07",
    ] {
        assert!(screen.contains(mark), "{mark:?} is missing from {screen:?}");
    }
    assert!(screen.contains("inside"));
    assert_eq!(output.end().exit, Some(7));
    assert!(!output.end().lost);
}
