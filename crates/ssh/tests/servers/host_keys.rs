use synt_ssh::{ConnectionState, SshError};
use synt_test_servers::{SshHome, SshServers};

use crate::Setup;

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_host_missing_from_known_hosts_is_refused_with_its_fingerprint() {
    let Some(setup) = Setup::knowing(|_| SshHome::empty()) else {
        return;
    };

    let refused = setup.run(&setup.bastion(), "true").await;

    let Err(SshError::HostKeyUnknown { host, fingerprint }) = refused else {
        panic!("expected an unknown host key, got {refused:?}");
    };
    assert_eq!(host, setup.servers.bastion_address());
    assert_eq!(fingerprint, setup.servers.bastion_key().fingerprint());
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_host_presenting_another_key_than_pinned_is_refused() {
    let pinned_wrong = |servers: &SshServers| {
        SshHome::empty().trusting(&servers.bastion_pattern(), servers.target_key())
    };
    let Some(setup) = Setup::knowing(pinned_wrong) else {
        return;
    };

    let refused = setup.run(&setup.bastion(), "true").await;

    let Err(SshError::HostKeyChanged { fingerprint, .. }) = refused else {
        panic!("expected a changed host key, got {refused:?}");
    };
    assert_eq!(fingerprint, setup.servers.bastion_key().fingerprint());
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn an_unknown_host_behind_a_known_bastion_is_refused() {
    let bastion_only = |servers: &SshServers| {
        SshHome::empty().trusting(&servers.bastion_pattern(), servers.bastion_key())
    };
    let Some(setup) = Setup::knowing(bastion_only) else {
        return;
    };

    let refused = setup.run(&setup.target(), "true").await;

    let Err(SshError::HostKeyUnknown { host, fingerprint }) = refused else {
        panic!("expected an unknown host key, got {refused:?}");
    };
    assert_eq!(host, SshServers::TARGET_NAME);
    assert_eq!(fingerprint, setup.servers.target_key().fingerprint());
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn warming_connects_only_to_hosts_already_in_known_hosts() {
    let Some(setup) = Setup::start() else { return };

    let warmed = setup.pool.warm(&setup.target()).await.unwrap();
    setup.home.forget_all();
    let refused = setup.pool.warm(&setup.bastion()).await;

    assert_eq!(warmed, ConnectionState::Connected);
    assert!(matches!(refused, Err(SshError::NotKnownHost { .. })));
}
