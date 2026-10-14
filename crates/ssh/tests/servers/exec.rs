use std::time::Duration;

use synt_ssh::{ExecLimits, SshError, exec};
use tokio::time;

use crate::Setup;
use crate::setup::seconds;

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_command_reports_its_exit_code_output_and_errors() {
    let Some(setup) = Setup::start() else { return };

    let output = setup
        .run(&setup.bastion(), "echo out; echo err >&2; exit 3")
        .await
        .unwrap();

    assert_eq!(output.exit, Some(3));
    assert_eq!(output.signal, None);
    assert_eq!(output.stdout, "out\n");
    assert_eq!(output.stderr, "err\n");
    assert!(!output.truncated);
    assert!(output.algorithms.is_some());
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_command_killed_by_a_signal_reports_the_signal() {
    let Some(setup) = Setup::start() else { return };

    let output = setup.run(&setup.bastion(), "kill -KILL $$").await.unwrap();

    assert_eq!(output.exit, None);
    assert_eq!(output.signal.as_deref(), Some("KILL"));
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn output_past_the_limit_is_cut_and_marked_truncated() {
    let Some(setup) = Setup::start() else { return };
    let limits = ExecLimits {
        output_bytes: 1000,
        ..ExecLimits::default()
    };

    let output = setup
        .run_within(
            &setup.bastion(),
            "head -c 50000 /dev/zero | tr '\\0' a; echo short >&2",
            limits,
        )
        .await
        .unwrap();

    assert_eq!(output.stdout, "a".repeat(1000));
    assert_eq!(output.stderr, "short\n");
    assert!(output.truncated);
    assert_eq!(output.exit, Some(0));
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_command_past_its_timeout_is_killed_on_the_server() {
    let Some(setup) = Setup::start() else { return };
    let bastion = setup.bastion();

    let timed_out = setup
        .run_within(&bastion, "exec sleep 31337", seconds(1))
        .await;

    assert!(matches!(
        timed_out,
        Err(SshError::CommandTimeout { ended: true, limit }) if limit == Duration::from_secs(1)
    ));

    let left = setup
        .run(&bastion, "pgrep -f 'sleep 3133[7]' || echo gone")
        .await
        .unwrap();
    assert_eq!(left.stdout, "gone\n");
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_stopped_call_halts_its_command_on_the_server() {
    let Some(setup) = Setup::start() else { return };
    let bastion = setup.bastion();
    let stop = time::sleep(Duration::from_millis(500));

    let stopped = exec(
        &setup.pool,
        &bastion,
        "exec sleep 31338",
        ExecLimits::default(),
        stop,
    )
    .await;

    assert!(matches!(stopped, Err(SshError::Stopped { ended: true })));

    let left = setup
        .run(&bastion, "pgrep -f 'sleep 3133[8]' || echo gone")
        .await
        .unwrap();
    assert_eq!(left.stdout, "gone\n");
}
