use synt_test_servers::{SshHome, SshServers};
use synt_testkit::TestError;

use crate::lab::BASTION;
use crate::{Lab, refusal};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn an_unknown_host_key_fails_the_call_and_the_journal_keeps_its_fingerprint()
-> Result<(), TestError> {
    let Some(lab) = Lab::knowing(|_| SshHome::empty()).await? else {
        return Ok(());
    };
    let fingerprint = lab.servers.bastion_key().fingerprint();

    let refused = refusal(lab.exec(BASTION, "true").await);
    let journaled = lab.last_failure("ssh.exec")?;

    assert!(refused.message.contains("is not in known_hosts"));
    assert!(refused.message.contains(&fingerprint));
    assert!(journaled["error"].as_str().unwrap().contains(&fingerprint));

    lab.stop().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_changed_host_key_fails_the_call_as_a_possible_interception() -> Result<(), TestError> {
    let pinned_wrong = |servers: &SshServers| {
        SshHome::empty().trusting(&servers.bastion_pattern(), servers.target_key())
    };
    let Some(lab) = Lab::knowing(pinned_wrong).await? else {
        return Ok(());
    };
    let fingerprint = lab.servers.bastion_key().fingerprint();

    let refused = refusal(lab.exec(BASTION, "true").await);
    let journaled = lab.last_failure("ssh.exec")?;

    assert!(refused.message.contains("does not match known_hosts"));
    assert!(refused.message.contains(&fingerprint));
    assert!(
        journaled["error"]
            .as_str()
            .unwrap()
            .contains("intercepting")
    );

    lab.stop().await?;

    Ok(())
}
