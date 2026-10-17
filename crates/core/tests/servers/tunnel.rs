use serde_json::json;
use synt_proto::ActionCall;
use synt_test_servers::SshServers;
use synt_testkit::TestError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::Lab;
use crate::lab::ECHO;

const GREETING: &[u8] = b"bytes through the core's tunnel\n";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_tunnel_resource_carries_bytes_and_both_ends_are_journaled() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    let opened = lab.call(ECHO, "tunnel.open", json!({})).await.unwrap();
    let local_port = u16::try_from(opened.value["local_port"].as_u64().unwrap()).unwrap();

    let mut stream = TcpStream::connect(("127.0.0.1", local_port)).await.unwrap();
    stream.write_all(GREETING).await.unwrap();
    let mut answer = vec![0; GREETING.len()];
    stream.read_exact(&mut answer).await.unwrap();
    drop(stream);

    let id = opened.value["id"].clone();
    let closed = lab
        .client
        .call(ActionCall::new("tunnel.close").params(json!({ "id": id })))
        .await
        .unwrap();
    let open_journal = lab.finished(opened.request).await?;
    let close_journal = lab.finished(closed.request).await?;

    assert_eq!(answer, GREETING);
    assert_eq!(open_journal["local_port"], json!(local_port));
    assert_eq!(open_journal["remote_port"], json!(SshServers::ECHO_PORT));
    assert_eq!(closed.value["sent_bytes"], json!(GREETING.len()));
    assert_eq!(closed.value["received_bytes"], json!(GREETING.len()));
    assert!(!close_journal.is_null());

    lab.stop().await?;

    Ok(())
}
