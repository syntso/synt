use std::time::Duration;

use synt_ssh::{ConnectionState, SshError};
use tokio::time;

use crate::Setup;

const GRACE: Duration = Duration::from_millis(300);

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn commands_to_one_host_share_one_connection() {
    let Some(setup) = Setup::start() else { return };
    let bastion = setup.bastion();

    let (first, second) = tokio::join!(setup.client_port(&bastion), setup.client_port(&bastion));

    assert_eq!(first, second);
    assert_eq!(setup.pool.connections().len(), 1);
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn an_unused_connection_closes_after_the_grace_period() {
    let Some(setup) = Setup::pooled(|_, pool| pool.grace(GRACE)) else {
        return;
    };
    let bastion = setup.bastion();

    let first = setup.client_port(&bastion).await;
    let held = setup.pool.connections();
    time::sleep(GRACE * 4).await;
    let after_grace = setup.pool.connections();
    let second = setup.client_port(&bastion).await;

    assert_eq!(held.len(), 1);
    assert_eq!(held[0].leases, 0);
    assert!(after_grace.is_empty());
    assert_ne!(first, second);
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_held_connection_outlives_the_grace_period() {
    let Some(setup) = Setup::pooled(|_, pool| pool.grace(GRACE)) else {
        return;
    };
    let bastion = setup.bastion();

    let lease = setup.pool.acquire(&bastion).await.unwrap();
    let first = setup.client_port(&bastion).await;
    time::sleep(GRACE * 4).await;
    let second = setup.client_port(&bastion).await;

    assert_eq!(first, second);
    assert_eq!(lease.state(), ConnectionState::Connected);
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_connection_the_server_drops_is_dialed_again() {
    let Some(setup) = Setup::start() else { return };
    let bastion = setup.bastion();

    let first = setup.client_port(&bastion).await;
    let dropped = setup.run(&bastion, "kill -KILL $PPID; sleep 5").await;
    let second = setup.client_port(&bastion).await;

    assert!(matches!(dropped, Err(SshError::ConnectionLost)));
    assert_ne!(first, second);
    assert_eq!(
        setup.pool.connections()[0].state,
        ConnectionState::Connected
    );
}
