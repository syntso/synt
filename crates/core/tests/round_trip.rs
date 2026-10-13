#![allow(clippy::unwrap_used)]

mod common;

use common::{Harness, kinds, ping};
use serde_json::json;
use synt_core::Pong;
use synt_journal::{ActionId, Actor, EventKind, Name, Seq};
use synt_proto::ProtocolVersion;

#[tokio::test(flavor = "multi_thread")]
async fn the_app_is_welcomed_with_the_current_protocol() {
    let harness = Harness::start();
    let client = harness.app().await;

    let welcome = client.welcome();

    assert_eq!(welcome.protocol, ProtocolVersion::CURRENT);
    assert_eq!(welcome.core, env!("CARGO_PKG_VERSION"));
}

#[tokio::test(flavor = "multi_thread")]
async fn ping_answers_with_its_own_request() {
    let harness = Harness::start();
    let client = harness.app().await;

    let outcome = client.call(ping()).await.unwrap();
    let pong: Pong = serde_json::from_value(outcome.value).unwrap();

    assert_eq!(pong.request, outcome.request);
    assert_eq!(pong.core, env!("CARGO_PKG_VERSION"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_request_is_the_seq_of_the_requested_event() {
    let harness = Harness::start();
    let client = harness.app().await;

    let outcome = client.call(ping()).await.unwrap();
    let trail = harness.trail(outcome.request);

    assert_eq!(trail[0].kind(), EventKind::Requested);
    assert_eq!(trail[0].seq(), Seq::new(outcome.request.0));
    assert_eq!(trail[0].call(), None);
    assert_eq!(trail[0].details().value(), &json!({ "params": null }));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_is_journaled_as_requested_started_finished() {
    let harness = Harness::start();
    let client = harness.app().await;

    let outcome = client.call(ping()).await.unwrap();
    let request = Seq::new(outcome.request.0);
    let trail = harness.trail(outcome.request);
    let ping_id = ActionId::new("core.ping").unwrap();

    assert_eq!(
        kinds(&trail),
        [
            EventKind::Requested,
            EventKind::Started,
            EventKind::Finished
        ]
    );
    assert!(trail[1..].iter().all(|event| event.call() == Some(request)));
    assert!(trail.windows(2).all(|pair| pair[0].seq() < pair[1].seq()));
    assert!(trail.iter().all(|event| event.actor() == &Actor::Person));
    assert!(trail.iter().all(|event| event.action() == &ping_id));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_journal_holds_nothing_but_the_call() {
    let harness = Harness::start();
    let client = harness.app().await;

    let outcome = client.call(ping()).await.unwrap();

    assert_eq!(harness.events(), harness.trail(outcome.request));
    assert!(harness.is_intact());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_is_journaled_under_its_own_name() {
    let harness = Harness::start();
    let client = harness.agent("deploy-bot").await;

    let outcome = client.call(ping()).await.unwrap();
    let trail = harness.trail(outcome.request);
    let agent = Actor::Agent(Name::new("deploy-bot").unwrap());

    assert_eq!(trail.len(), 3);
    assert!(trail.iter().all(|event| event.actor() == &agent));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_connection_keeps_its_own_actor() {
    let harness = Harness::start();
    let person = harness.app().await;
    let agent = harness.agent("reviewer").await;

    let by_person = person.call(ping()).await.unwrap();
    let by_agent = agent.call(ping()).await.unwrap();

    let person_trail = harness.trail(by_person.request);
    let agent_trail = harness.trail(by_agent.request);
    let reviewer = Actor::Agent(Name::new("reviewer").unwrap());

    assert!(
        person_trail
            .iter()
            .all(|event| event.actor() == &Actor::Person)
    );
    assert!(agent_trail.iter().all(|event| event.actor() == &reviewer));
}
