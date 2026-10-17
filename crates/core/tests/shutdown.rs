use std::time::Duration;

use synt_actions::Registry;
use synt_client::{Autostart, Client, ClientError};
use synt_core::{Broker, Core, Ping, Server};
use synt_platform::{AppCheck, LocalStream};
use synt_proto::RequestSeq;
use synt_testkit::TestError;
use synt_testkit::core::{Harness, RunningServer, app_hello, ping};
use tokio::time;

const ANSWER_LIMIT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn run_returns_once_the_shutdown_future_completes() -> Result<(), TestError> {
    let mut harness = Harness::start()?;
    let client = harness.app().await?;
    client.call(ping()).await.unwrap();

    harness.stop().await?;

    assert!(LocalStream::connect(harness.endpoint()).await.is_err());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn stopping_disconnects_the_clients_still_connected() -> Result<(), TestError> {
    let mut harness = Harness::start()?;
    let client = harness.app().await?;

    harness.stop().await?;
    let after = time::timeout(ANSWER_LIMIT, client.call(ping()))
        .await
        .unwrap();

    assert!(matches!(after, Err(ClientError::Disconnected)));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_endpoint_cannot_be_bound_twice() -> Result<(), TestError> {
    let harness = Harness::start()?;

    let second = Server::bind(
        harness.endpoint(),
        core(&harness)?,
        AppCheck::for_this_core().unwrap(),
    );

    assert!(second.is_err());

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_endpoint_can_be_bound_again_after_shutdown() -> Result<(), TestError> {
    let mut harness = Harness::start()?;
    harness.stop().await?;

    let restarted = RunningServer::spawn(harness.endpoint(), core(&harness)?)?;
    let client = Client::connect(harness.endpoint(), app_hello(), &Autostart::off())
        .await
        .unwrap();
    let outcome = client.call(ping()).await.unwrap();

    assert_eq!(outcome.request, RequestSeq::new(1));
    restarted.stop().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_journal_stays_intact_across_a_restart() -> Result<(), TestError> {
    let mut harness = Harness::start()?;
    harness.app().await?.call(ping()).await.unwrap();
    harness.stop().await?;

    let restarted = RunningServer::spawn(harness.endpoint(), core(&harness)?)?;
    let client = Client::connect(harness.endpoint(), app_hello(), &Autostart::off())
        .await
        .unwrap();
    let outcome = client.call(ping()).await.unwrap();
    restarted.stop().await?;

    assert_eq!(outcome.request, RequestSeq::new(4));
    assert_eq!(harness.events()?.len(), 6);
    assert!(harness.is_intact()?);

    Ok(())
}

fn core(harness: &Harness) -> Result<Core, TestError> {
    let mut registry = Registry::new();
    registry.register(Ping)?;

    Ok(Core::new(registry, harness.journal(), Broker::new()))
}
