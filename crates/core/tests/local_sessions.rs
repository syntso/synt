use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use serde_json::{Value, json};
use synt_actions::Registry;
use synt_client::{Autostart, Client, StreamEvent};
use synt_core::{
    Broker, Core, LocalSessionOpen, SessionBlocks, SessionIntegration, SessionList, Terminals,
};
use synt_journal::{Event, EventKind, Filter, Journal, JournalWriter, MemoryKeyStore, Seq};
use synt_platform::Endpoint;
use synt_proto::{ActionCall, ClientKind, EndReason, ErrorCode, Hello, StreamId};
use synt_testkit::TestError;
use synt_testkit::core::{RunningServer, app_hello, rejection};
use tempfile::TempDir;
use tokio::time;

const LIMIT: Duration = Duration::from_secs(20);
const PAGE: u32 = 1000;

fn start() -> Result<(TempDir, Endpoint, JournalWriter, RunningServer), TestError> {
    let directory = tempfile::tempdir()?;
    let unique = directory
        .path()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let endpoint = Endpoint::named(directory.path(), &format!("synt-local-{unique}"));

    let keys = Arc::new(MemoryKeyStore::new()?);
    let journal = Journal::create(directory.path().join("journal.sqlite"), keys)?;
    let journal = JournalWriter::spawn(journal)?;

    let terminals = Terminals::new(journal.clone());
    let mut registry = Registry::new();
    registry.register(LocalSessionOpen::new(terminals.clone()))?;
    registry.register(SessionBlocks::new(terminals.clone()))?;
    registry.register(SessionList::new(terminals))?;
    registry.register(SessionIntegration)?;

    let core = Core::new(registry, journal.clone(), Broker::new());
    let server = RunningServer::spawn(&endpoint, core)?;

    Ok((directory, endpoint, journal, server))
}

async fn connect(endpoint: &Endpoint, hello: Hello) -> Result<Client, TestError> {
    Ok(Client::connect(endpoint, hello, &Autostart::off()).await?)
}

async fn finished_event(journal: &JournalWriter, request: Seq) -> Result<Event, TestError> {
    loop {
        let reader = journal.reader()?;
        let trail = reader.page(&Filter::all().call(request), None, PAGE)?;

        if let Some(finished) = trail
            .into_iter()
            .find(|event| event.kind() == EventKind::Finished)
        {
            return Ok(finished);
        }

        time::sleep(Duration::from_millis(20)).await;
    }
}

fn open_call() -> ActionCall {
    ActionCall::new("local.session.open").params(json!({ "cols": 80, "rows": 24 }))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_shell_runs_until_it_exits_and_is_journaled() -> Result<(), TestError> {
    let (_directory, endpoint, journal, server) = start()?;
    let client = connect(&endpoint, app_hello()).await?;

    let outcome = client.call(open_call()).await.unwrap();
    let opened: Value = outcome.value.clone();
    let stream: StreamId = serde_json::from_value(opened["stream"].clone()).unwrap();
    let mut output = client.stream(stream);

    let listed = client.call(ActionCall::new("session.list")).await.unwrap();
    assert_eq!(listed.value[0]["kind"], json!("local"));
    assert_eq!(listed.value[0]["target"], Value::Null);

    client
        .send_stream(stream, Bytes::from_static(b"exit 4\r"))
        .await
        .unwrap();

    let ended = time::timeout(LIMIT, async {
        loop {
            match output.next().await {
                Some(StreamEvent::Data(_)) => {}
                Some(StreamEvent::Ended(ended)) => return ended,
                None => panic!("the connection closed before the session ended"),
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(ended.reason, EndReason::Exited);
    assert_eq!(ended.exit, Some(4));
    assert_eq!(ended.session.get(), opened["session"].as_u64().unwrap());

    let request = Seq::new(outcome.request.get());
    let finished = time::timeout(LIMIT, finished_event(&journal, request)).await??;

    assert_eq!(finished.action().as_str(), "local.session.open");
    assert_eq!(finished.details().value()["exit"], json!(4));
    assert_eq!(finished.details().value()["blocks"], json!(0));

    server.stop().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_person_opens_a_local_shell_or_reads_its_blocks() -> Result<(), TestError> {
    let (_directory, endpoint, _journal, server) = start()?;
    let agent = ClientKind::Agent {
        name: "helper".to_owned(),
    };
    let client = connect(&endpoint, Hello::new(agent, "synt-tests")).await?;

    let opened = rejection(client.call(open_call()).await);
    let blocks = rejection(
        client
            .call(ActionCall::new("session.blocks").params(json!({ "session": 1 })))
            .await,
    );

    assert_eq!(opened.code, ErrorCode::DENIED);
    assert_eq!(blocks.code, ErrorCode::DENIED);

    server.stop().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_integration_snippets_are_offered_for_three_shells() -> Result<(), TestError> {
    let (_directory, endpoint, _journal, server) = start()?;
    let client = connect(&endpoint, app_hello()).await?;

    let all = client
        .call(ActionCall::new("session.integration"))
        .await
        .unwrap();
    let zsh = client
        .call(ActionCall::new("session.integration").params(json!({ "shell": "zsh" })))
        .await
        .unwrap();

    let shells: Vec<&str> = all
        .value
        .as_array()
        .unwrap()
        .iter()
        .map(|snippet| snippet["shell"].as_str().unwrap())
        .collect();
    assert_eq!(shells, ["bash", "zsh", "fish"]);
    assert_eq!(zsh.value[0]["startup_file"], json!("~/.zshrc"));
    assert!(zsh.value[0]["snippet"].as_str().unwrap().contains("133;D"));

    server.stop().await?;

    Ok(())
}
