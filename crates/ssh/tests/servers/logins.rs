use std::fs;
use std::sync::Arc;

use synt_ssh::{PoolBuilder, SshError, StoredKey};
use synt_test_servers::SshServers;

use crate::{FileVault, Setup};

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_key_file_logs_in_without_an_agent() {
    let Some(setup) = Setup::start() else { return };

    let output = setup.run(&setup.bastion(), "whoami").await.unwrap();

    assert_eq!(output.output.stdout, "synt\n");
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_refused_key_file_is_named_as_the_only_key_tried() {
    let Some(setup) = Setup::start() else { return };
    let stranger = tempfile::NamedTempFile::new().unwrap();
    let key = StoredKey::generate_ed25519("stranger").unwrap();
    fs::write(stranger.path(), key.private_bytes()).unwrap();

    let bastion = setup
        .bastion()
        .with_identity_files(vec![stranger.path().to_owned()]);
    let refused = setup.run(&bastion, "true").await;

    let Err(SshError::NoAuthentication { tried }) = refused else {
        panic!("expected the login to be refused, got {refused:?}");
    };
    assert_eq!(tried, vec![stranger.path().display().to_string()]);
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_vault_key_logs_in_by_its_secret_name() {
    let Some(setup) = Setup::pooled(with_vault) else {
        return;
    };

    let bastion = setup.bastion().with_vault_key("night");
    let output = setup.run(&bastion, "whoami").await.unwrap();

    assert_eq!(output.output.stdout, "synt\n");
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_vault_key_that_is_not_there_is_refused_by_name() {
    let Some(setup) = Setup::pooled(with_vault) else {
        return;
    };

    let bastion = setup.bastion().with_vault_key("day");
    let refused = setup.run(&bastion, "true").await;

    let Err(SshError::VaultKey { name, reason }) = refused else {
        panic!("expected the vault key to be missing, got {refused:?}");
    };
    assert_eq!(name, "day");
    assert!(reason.contains("no such key"));
}

fn with_vault(servers: &SshServers, pool: PoolBuilder) -> PoolBuilder {
    let vault = FileVault::holding("night", servers.vault_key());

    pool.key_vault(Arc::new(vault))
}
