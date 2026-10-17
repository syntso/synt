use serde_json::{Value, json};
use synt_journal::{
    Actor, Details, Event, EventKind, Filter, Name, NewEvent, ResourceRef, Seq, Timestamp,
};
use synt_testkit::TestError;
use synt_testkit::journal::{Fixture, action, exec};

#[test]
fn appended_events_come_back_unchanged() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.create()?;

    let request = NewEvent::new(
        Actor::Agent(Name::new("agent-1").unwrap()),
        EventKind::Requested,
        action("ssh.exec")?,
    )
    .resource(ResourceRef::new("@host/api-1").unwrap())
    .details(Details::from_serialize(&json!({ "command": "systemctl restart api" })).unwrap());

    let seq = journal.append(&request).unwrap();
    let event = journal.reader().unwrap().get(seq).unwrap().unwrap();

    assert_eq!(seq, Seq::FIRST);
    assert_eq!(event.actor(), request.actor());
    assert_eq!(event.kind(), EventKind::Requested);
    assert_eq!(event.action().as_str(), "ssh.exec");
    assert_eq!(
        event.resource().map(ResourceRef::as_str),
        Some("@host/api-1")
    );
    assert_eq!(
        event.details().value(),
        &json!({ "command": "systemctl restart api" })
    );
    assert_eq!(event.at(), Timestamp::from_millis(1_700_000_000_000));

    Ok(())
}

#[test]
fn sequence_numbers_are_consecutive() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.create()?;

    let first = journal.append(&exec("uptime")?).unwrap();
    let batch = journal
        .append_all(&[exec("df -h")?, exec("free -m")?, exec("w")?])
        .unwrap();

    assert_eq!(first, Seq::new(1));
    assert_eq!(batch, [Seq::new(2), Seq::new(3), Seq::new(4)]);
    assert_eq!(journal.anchors().head.seq, Seq::new(4));

    Ok(())
}

#[test]
fn an_empty_batch_changes_nothing() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.create()?;

    assert!(journal.append_all(&[]).unwrap().is_empty());
    assert_eq!(journal.anchors().head.seq, Seq::ORIGIN);

    Ok(())
}

#[test]
fn a_reopened_journal_continues_the_chain() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let id = fixture.with_events(3)?;

    let mut journal = fixture.open(id).unwrap();
    let next = journal.append(&exec("after reopen")?).unwrap();

    assert_eq!(next, Seq::new(4));
    assert!(journal.reader().unwrap().verify().unwrap().is_intact());

    Ok(())
}

#[test]
fn time_never_runs_backwards() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.create()?;

    let first = journal.append(&exec("before")?).unwrap();
    fixture.clock.set(1_000);
    let second = journal
        .append(&exec("after the clock jumped back")?)
        .unwrap();

    let reader = journal.reader().unwrap();
    let first_at = reader.get(first).unwrap().unwrap().at();
    let second_at = reader.get(second).unwrap().unwrap().at();

    assert!(second_at >= first_at);

    Ok(())
}

#[test]
fn pages_follow_the_filter_and_continue_after_the_last_seq() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.create()?;

    let request = journal.append(&exec("deploy")?).unwrap();
    let started = NewEvent::new(Actor::Core, EventKind::Started, action("ssh.exec")?).call(request);
    let finished =
        NewEvent::new(Actor::Core, EventKind::Finished, action("ssh.exec")?).call(request);
    journal
        .append_all(&[exec("unrelated")?, started, finished])
        .unwrap();

    let reader = journal.reader().unwrap();
    let call = Filter::all().call(request);

    let first_page = reader.page(&call, None, 2).unwrap();
    let second_page = reader
        .page(&call, first_page.last().map(Event::seq), 2)
        .unwrap();

    let kinds: Vec<_> = first_page
        .iter()
        .chain(&second_page)
        .map(Event::kind)
        .collect();
    assert_eq!(
        kinds,
        [
            EventKind::Requested,
            EventKind::Started,
            EventKind::Finished
        ]
    );

    let only_finished = reader
        .page(&Filter::all().kind(EventKind::Finished), None, 10)
        .unwrap();
    assert_eq!(only_finished.len(), 1);

    Ok(())
}

#[test]
fn pages_filter_by_actor_resource_and_time() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let mut journal = fixture.create()?;

    journal.append(&exec("as a person")?).unwrap();
    fixture.clock.advance(60_000);
    journal
        .append(&NewEvent::new(
            Actor::Core,
            EventKind::Note,
            action("core.start")?,
        ))
        .unwrap();

    let reader = journal.reader().unwrap();
    let count = |filter: Filter| reader.page(&filter, None, 100).unwrap().len();

    assert_eq!(count(Filter::all()), 2);
    assert_eq!(count(Filter::all().actor(Actor::Core)), 1);
    assert_eq!(
        count(Filter::all().resource(ResourceRef::new("@host/api-1").unwrap())),
        1
    );
    assert_eq!(
        count(Filter::all().since(Timestamp::from_millis(1_700_000_030_000))),
        1
    );
    assert_eq!(
        count(Filter::all().until(Timestamp::from_millis(1_700_000_030_000))),
        1
    );

    Ok(())
}

#[test]
fn export_writes_one_json_line_per_event() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let id = fixture.with_events(3)?;

    let mut out = Vec::new();
    let written = fixture
        .reader(id)?
        .export_jsonl(&Filter::all(), &mut out)
        .unwrap();

    let lines: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(written, 3);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["seq"], 1);
    assert_eq!(lines[0]["actor"], json!({ "type": "person" }));
    assert_eq!(lines[2]["details"], json!({ "command": "step 3" }));
    assert_eq!(lines[2]["link"].as_str().map(str::len), Some(64));

    Ok(())
}
