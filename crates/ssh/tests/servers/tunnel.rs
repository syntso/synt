use std::time::Duration;

use synt_ssh::{Destination, Tunnels};
use synt_test_servers::SshServers;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::Setup;

const GREETING: &[u8] = b"bytes through the tunnel\n";

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_tunnel_carries_bytes_both_ways() {
    let Some(setup) = Setup::start() else { return };
    let tunnels = Tunnels::new(setup.pool.clone());
    let echo = Destination::new("127.0.0.1", SshServers::ECHO_PORT).unwrap();

    let opened = tunnels
        .open(&setup.target(), echo, None, CancellationToken::new())
        .await
        .unwrap();

    let mut stream = TcpStream::connect(("127.0.0.1", opened.local_port))
        .await
        .unwrap();
    stream.write_all(GREETING).await.unwrap();
    let mut answer = vec![0; GREETING.len()];
    stream.read_exact(&mut answer).await.unwrap();
    drop(stream);

    let closed = tunnels.close(opened.id).await.unwrap();

    assert_eq!(answer, GREETING);
    assert_eq!(closed.connections, 1);
    assert_eq!(closed.sent_bytes, GREETING.len() as u64);
    assert_eq!(closed.received_bytes, GREETING.len() as u64);
    assert!(tunnels.list().is_empty());
}

#[tokio::test]
#[ignore = "needs the ssh test servers: tests/ssh/up.sh, then SYNT_SSH_TESTS=1"]
async fn a_cancelled_tunnel_stops_listening() {
    let Some(setup) = Setup::start() else { return };
    let tunnels = Tunnels::new(setup.pool.clone());
    let stop = CancellationToken::new();
    let echo = Destination::new("127.0.0.1", SshServers::ECHO_PORT).unwrap();

    let opened = tunnels
        .open(&setup.bastion(), echo, None, stop.clone())
        .await
        .unwrap();
    stop.cancel();

    let closing = async {
        while !tunnels.list().is_empty() {
            time::sleep(Duration::from_millis(20)).await;
        }
    };
    time::timeout(Duration::from_secs(5), closing)
        .await
        .unwrap();

    assert!(
        TcpStream::connect(("127.0.0.1", opened.local_port))
            .await
            .is_err()
    );
}
