#![allow(clippy::unwrap_used)]

mod common;

use std::time::Duration;

use common::{Harness, RunningServer, app_hello, ping};
use synt_actions::Registry;
use synt_client::{Client, ClientError};
use synt_core::{Broker, Core, Ping, Server};
use synt_platform::LocalStream;
use synt_proto::RequestSeq;
use tokio::time;

const ANSWER_LIMIT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn run_returns_once_the_shutdown_future_completes() {
    let mut harness = Harness::start();
    let client = harness.app().await;
    client.call(ping()).await.unwrap();

    harness.stop().await;

    assert!(LocalStream::connect(harness.endpoint()).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn stopping_disconnects_the_clients_still_connected() {
    let mut harness = Harness::start();
    let client = harness.app().await;

    harness.stop().await;
    let after = time::timeout(ANSWER_LIMIT, client.call(ping()))
        .await
        .unwrap();

    assert!(matches!(after, Err(ClientError::Disconnected)));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_endpoint_cannot_be_bound_twice() {
    let harness = Harness::start();

    let second = Server::bind(harness.endpoint(), core(&harness));

    assert!(second.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_endpoint_can_be_bound_again_after_shutdown() {
    let mut harness = Harness::start();
    harness.stop().await;

    let restarted = RunningServer::spawn(harness.endpoint(), core(&harness)).unwrap();
    let client = Client::connect(harness.endpoint(), app_hello())
        .await
        .unwrap();
    let outcome = client.call(ping()).await.unwrap();

    assert_eq!(outcome.request, RequestSeq(1));
    restarted.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_journal_stays_intact_across_a_restart() {
    let mut harness = Harness::start();
    harness.app().await.call(ping()).await.unwrap();
    harness.stop().await;

    let restarted = RunningServer::spawn(harness.endpoint(), core(&harness)).unwrap();
    let client = Client::connect(harness.endpoint(), app_hello())
        .await
        .unwrap();
    let outcome = client.call(ping()).await.unwrap();
    restarted.stop().await;

    assert_eq!(outcome.request, RequestSeq(4));
    assert_eq!(harness.events().len(), 6);
    assert!(harness.is_intact());
}

fn core(harness: &Harness) -> Core {
    let mut registry = Registry::new();
    registry.register(Ping).unwrap();

    Core::new(registry, harness.journal(), Broker::new())
}
