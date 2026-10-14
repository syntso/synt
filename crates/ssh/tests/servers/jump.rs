use synt_ssh::ConnectionState;
use synt_test_servers::SshServers;

use crate::Setup;

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_host_behind_the_bastion_is_reached_through_it() {
    let Some(setup) = Setup::start() else { return };

    let direct = setup.run(&setup.bastion(), "hostname").await.unwrap();
    let jumped = setup.run(&setup.target(), "hostname").await.unwrap();

    assert_eq!(jumped.exit, Some(0));
    assert_ne!(jumped.stdout, direct.stdout);

    let connections = setup.pool.connections();
    let target = connections
        .iter()
        .find(|status| status.target == format!("{}@{}", SshServers::USER, SshServers::TARGET_NAME))
        .unwrap();

    assert_eq!(connections.len(), 2);
    assert_eq!(target.via, Some(setup.bastion().label()));
    assert_eq!(target.state, ConnectionState::Connected);
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn the_jump_shares_the_bastion_connection_with_direct_commands() {
    let Some(setup) = Setup::start() else { return };
    let bastion = setup.bastion();

    let before = setup.client_port(&bastion).await;
    setup.run(&setup.target(), "true").await.unwrap();
    let after = setup.client_port(&bastion).await;

    assert_eq!(before, after);
}
