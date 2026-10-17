use std::collections::BTreeSet;

use synt_client::Client;
use synt_core::Pong;
use synt_journal::{Actor, EventKind, Filter, Name};
use synt_proto::{ActionOutcome, RequestSeq};
use synt_testkit::TestError;
use synt_testkit::core::{Harness, ping};
use tokio::task::JoinSet;

const CALLS: usize = 200;
const CONNECTIONS: usize = 8;

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_calls_on_one_connection_all_succeed() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let client = harness.app().await?;

    let outcomes = ping_concurrently(&[client], CALLS).await?;

    assert_answered(&harness, &outcomes)?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_calls_on_several_connections_all_succeed() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let mut clients = Vec::new();

    for _ in 0..CONNECTIONS {
        clients.push(harness.app().await?);
    }

    let outcomes = ping_concurrently(&clients, CALLS / CONNECTIONS).await?;

    assert_answered(&harness, &outcomes)?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_actors_are_never_mixed_up() -> Result<(), TestError> {
    let harness = Harness::start()?;
    let person = harness.app().await?;
    let agent = harness.agent("load-test").await?;

    let outcomes = ping_concurrently(&[person, agent], CALLS / 2).await?;

    let by_agent = Filter::all()
        .actor(Actor::Agent(Name::new("load-test").unwrap()))
        .kind(EventKind::Finished);
    let by_person = Filter::all().actor(Actor::Person).kind(EventKind::Finished);

    assert_answered(&harness, &outcomes)?;
    assert_eq!(harness.matching(&by_agent)?.len(), CALLS / 2);
    assert_eq!(harness.matching(&by_person)?.len(), CALLS / 2);

    Ok(())
}

async fn ping_concurrently(
    clients: &[Client],
    per_client: usize,
) -> Result<Vec<ActionOutcome>, TestError> {
    let mut calls = JoinSet::new();

    for client in clients {
        for _ in 0..per_client {
            let client = client.clone();
            calls.spawn(async move { client.call(ping()).await });
        }
    }

    let outcomes = calls
        .join_all()
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;

    Ok(outcomes)
}

fn assert_answered(harness: &Harness, outcomes: &[ActionOutcome]) -> Result<(), TestError> {
    let requests: BTreeSet<RequestSeq> = outcomes.iter().map(|outcome| outcome.request).collect();
    let finished = harness.matching(&Filter::all().kind(EventKind::Finished))?;

    assert_eq!(outcomes.len(), CALLS);
    assert_eq!(requests.len(), CALLS);

    for outcome in outcomes {
        let pong: Pong = serde_json::from_value(outcome.value.clone())?;
        assert_eq!(pong.request, outcome.request);
    }

    assert_eq!(finished.len(), CALLS);
    assert_eq!(harness.events()?.len(), CALLS * 3);
    assert!(harness.is_intact()?);

    Ok(())
}
