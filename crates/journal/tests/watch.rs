#![allow(clippy::unwrap_used)]

mod common;

use common::{Fixture, action, exec};
use synt_journal::{Actor, EventKind, JournalWriter, NewEvent, Seq};

#[tokio::test]
async fn a_watcher_hears_every_event_after_it_is_committed() {
    let fixture = Fixture::new();
    let writer = JournalWriter::spawn(fixture.create()).unwrap();
    let mut watcher = writer.watch();

    let request = writer
        .open_call(exec("uptime"), vec![started()])
        .await
        .unwrap();
    writer.append_async(exec("whoami")).await.unwrap();

    let heard: Vec<_> = (0..3).map(|_| watcher.try_recv().unwrap()).collect();

    assert_eq!(
        heard.iter().map(|event| event.seq).collect::<Vec<_>>(),
        [request, request.next(), request.next().next()]
    );
    assert_eq!(heard[0].kind, EventKind::Requested);
    assert_eq!(heard[1].kind, EventKind::Started);
    assert_eq!(heard[1].call, Some(request));
    assert_eq!(heard[2].call, None);
    assert_eq!(heard[0].actor, Actor::Person);
    assert_eq!(heard[0].action.as_str(), "ssh.exec");
}

#[test]
fn appends_go_on_without_watchers() {
    let fixture = Fixture::new();
    let writer = JournalWriter::spawn(fixture.create()).unwrap();
    drop(writer.watch());

    assert_eq!(writer.append(exec("ls")).unwrap(), Seq::FIRST);
}

fn started() -> NewEvent {
    NewEvent::new(Actor::Person, EventKind::Started, action("ssh.exec"))
}
