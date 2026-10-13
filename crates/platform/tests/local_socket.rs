#![allow(clippy::unwrap_used)]

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::net::UnixListener;

use synt_platform::{Endpoint, LocalListener, LocalStream};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn endpoint(directory: &TempDir) -> Endpoint {
    let unique = directory.path().file_name().unwrap().to_string_lossy();
    Endpoint::named(directory.path(), &format!("synt-test-{unique}"))
}

#[tokio::test]
async fn bytes_travel_both_ways() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);
    let mut listener = LocalListener::bind(&endpoint).unwrap();

    let server = tokio::spawn(async move {
        let mut stream = listener.accept().await.unwrap();
        let mut received = [0; 4];

        stream.read_exact(&mut received).await.unwrap();
        stream.write_all(b"pong").await.unwrap();

        received
    });

    let mut client = LocalStream::connect(&endpoint).await.unwrap();
    client.write_all(b"ping").await.unwrap();

    let mut reply = [0; 4];
    client.read_exact(&mut reply).await.unwrap();

    assert_eq!(&reply, b"pong");
    assert_eq!(&server.await.unwrap(), b"ping");
}

#[tokio::test]
async fn several_clients_are_accepted_one_after_another() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);
    let mut listener = LocalListener::bind(&endpoint).unwrap();

    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let mut stream = listener.accept().await.unwrap();
            stream.write_all(b"hi").await.unwrap();
        }
    });

    for _ in 0..3 {
        let mut client = LocalStream::connect(&endpoint).await.unwrap();
        let mut greeting = [0; 2];

        client.read_exact(&mut greeting).await.unwrap();
        assert_eq!(&greeting, b"hi");
    }

    server.await.unwrap();
}

#[tokio::test]
async fn a_second_listener_on_the_same_endpoint_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);

    let _first = LocalListener::bind(&endpoint).unwrap();
    let second = LocalListener::bind(&endpoint);

    assert!(second.is_err());
}

#[tokio::test]
async fn a_client_that_leaves_at_once_does_not_block_the_next() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);
    let mut listener = LocalListener::bind(&endpoint).unwrap();

    let server = tokio::spawn(async move {
        loop {
            let Ok(mut stream) = listener.accept().await else {
                continue;
            };

            if stream.write_all(b"hi").await.is_ok() {
                let mut ack = [0; 2];

                if stream.read_exact(&mut ack).await.is_ok() {
                    return;
                }
            }
        }
    });

    drop(LocalStream::connect(&endpoint).await.unwrap());

    let mut client = LocalStream::connect(&endpoint).await.unwrap();
    let mut greeting = [0; 2];

    client.read_exact(&mut greeting).await.unwrap();
    client.write_all(b"ok").await.unwrap();

    assert_eq!(&greeting, b"hi");
    server.await.unwrap();
}

#[tokio::test]
async fn connecting_to_nothing_fails() {
    let directory = tempfile::tempdir().unwrap();
    let refused = LocalStream::connect(&endpoint(&directory)).await;

    assert!(refused.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn a_stale_socket_file_is_replaced() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);

    let stale = UnixListener::bind(endpoint.address()).unwrap();
    drop(stale);

    assert!(LocalListener::bind(&endpoint).is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn a_file_that_is_not_a_socket_is_left_alone() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);

    fs::write(endpoint.address(), b"keep me").unwrap();

    assert!(LocalListener::bind(&endpoint).is_err());
    assert_eq!(fs::read(endpoint.address()).unwrap(), b"keep me");
}

#[cfg(unix)]
#[tokio::test]
async fn a_shared_directory_is_made_private() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = endpoint(&directory);

    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let _listener = LocalListener::bind(&endpoint).unwrap();

    let mode = fs::metadata(directory.path()).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
}
