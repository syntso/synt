use serde_json::json;
use synt_core::KeyName;
use synt_ssh::StoredKey;

use crate::lab::{BASTION, VAULTED};
use crate::{Lab, refusal};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_key_file_named_by_the_resource_logs_in_without_an_agent() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let outcome = lab.exec(BASTION, "whoami").await.unwrap();

    assert_eq!(outcome.value["stdout"], json!("synt\n"));

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_vault_key_logs_in_through_its_secret_reference() {
    let Some(lab) = Lab::start().await else {
        return;
    };
    let key = StoredKey::import(&lab.servers.vault_key()).await.unwrap();
    lab.keys
        .store(&KeyName::parse("night").unwrap(), &key)
        .unwrap();

    let outcome = lab.exec(VAULTED, "whoami").await.unwrap();
    let finished = lab.finished(outcome.request).await;

    assert_eq!(outcome.value["stdout"], json!("synt\n"));
    assert_eq!(finished["exit"], json!(0));

    lab.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_secret_reference_to_a_missing_key_fails_by_its_name() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let refused = refusal(lab.exec(VAULTED, "true").await);
    let journaled = lab.last_failure("ssh.exec");

    assert!(refused.message.contains("secret://night"));
    assert_eq!(journaled["error"], json!(refused.message));

    lab.stop().await;
}
