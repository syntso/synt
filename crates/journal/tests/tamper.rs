#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::sync::Arc;

use common::{Fixture, exec};
use synt_journal::{
    Damage, DeviceKey, Journal, JournalError, JournalReader, KeyStore, MemoryKeyStore, Seq,
    Verification,
};

fn broken(at: u64, damage: Damage) -> Verification {
    Verification::Broken {
        at: Seq::new(at),
        damage,
    }
}

fn tampered(fixture: &Fixture, id: synt_journal::JournalId, sql: &str) -> Verification {
    fixture.raw().execute_batch(sql).unwrap();
    fixture.reader(id).verify().unwrap()
}

#[test]
fn an_untouched_journal_is_intact() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    assert_eq!(
        fixture.reader(id).verify().unwrap(),
        Verification::Intact { checked: 5 }
    );
}

#[test]
fn the_database_refuses_updates() {
    let fixture = Fixture::new();
    fixture.with_events(1);

    let connection = rusqlite::Connection::open(&fixture.path).unwrap();
    let refused = connection.execute("UPDATE events SET action = 'forged' WHERE seq = 1", []);

    assert!(refused.is_err());
}

#[test]
fn an_edited_detail_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let source = fixture
        .raw()
        .query_row("SELECT details FROM events WHERE seq = 2", [], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .unwrap();
    fixture
        .raw()
        .execute("UPDATE events SET details = ?1 WHERE seq = 3", [source])
        .unwrap();

    assert_eq!(
        fixture.reader(id).verify().unwrap(),
        broken(3, Damage::Altered)
    );
}

#[test]
fn an_edited_time_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(
        &fixture,
        id,
        "UPDATE events SET at = at - 3600000 WHERE seq = 4",
    );
    assert_eq!(verification, broken(4, Damage::Altered));
}

#[test]
fn an_edited_actor_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(
        &fixture,
        id,
        "UPDATE events SET actor = X'02' WHERE seq = 1",
    );
    assert_eq!(verification, broken(1, Damage::Altered));
}

#[test]
fn a_rewritten_link_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(
        &fixture,
        id,
        "UPDATE events SET link = zeroblob(32) WHERE seq = 2",
    );
    assert_eq!(verification, broken(2, Damage::Altered));
}

#[test]
fn a_removed_event_in_the_middle_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(&fixture, id, "DELETE FROM events WHERE seq = 3");
    assert_eq!(verification, broken(3, Damage::Missing));
}

#[test]
fn reordered_events_are_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(
        &fixture,
        id,
        "UPDATE events SET seq = 100 WHERE seq = 2;
         UPDATE events SET seq = 2 WHERE seq = 4;
         UPDATE events SET seq = 4 WHERE seq = 100;",
    );

    assert_eq!(verification, broken(2, Damage::Altered));
}

#[test]
fn a_cut_off_tail_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(&fixture, id, "DELETE FROM events WHERE seq >= 4");
    assert_eq!(
        verification,
        Verification::Truncated {
            last: Seq::new(3),
            head: Seq::new(5)
        }
    );

    let reopened = fixture.open(id);
    assert!(matches!(
        reopened,
        Err(JournalError::Tampered(Verification::Truncated { .. }))
    ));
}

#[test]
fn an_emptied_journal_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    let verification = tampered(&fixture, id, "DELETE FROM events");
    assert_eq!(
        verification,
        Verification::Truncated {
            last: Seq::ORIGIN,
            head: Seq::new(5)
        }
    );
}

#[test]
fn a_forged_event_at_the_end_is_refused_on_open() {
    let fixture = Fixture::new();
    let id = fixture.with_events(2);

    fixture
        .raw()
        .execute_batch(
            "INSERT INTO events (seq, at, actor, kind, call, action, resource, details, link)
             SELECT 3, at, actor, kind, call, action, resource, details, link FROM events WHERE seq = 2",
        )
        .unwrap();

    let reopened = fixture.open(id);
    assert!(
        matches!(reopened, Err(JournalError::Tampered(Verification::Broken { at, damage: Damage::Altered })) if at == Seq::new(3))
    );
}

#[test]
fn an_older_copy_of_the_file_is_found() {
    let fixture = Fixture::new();
    let id = fixture.with_events(3);

    let copy = fixture.dir.path().join("copy.sqlite");
    fs::copy(&fixture.path, &copy).unwrap();

    let mut journal = fixture.open(id).unwrap();
    journal.append(&exec("four")).unwrap();
    journal.append(&exec("five")).unwrap();
    drop(journal);

    fs::copy(&copy, &fixture.path).unwrap();

    let reopened = fixture.open(id);
    assert!(matches!(
        reopened,
        Err(JournalError::Tampered(Verification::Truncated { last, head })) if last == Seq::new(3) && head == Seq::new(5)
    ));
}

#[test]
fn another_journal_in_its_place_is_refused() {
    let fixture = Fixture::new();
    let id = fixture.with_events(2);

    let other_path = fixture.dir.path().join("other.sqlite");
    let mut other = Journal::create(&other_path, fixture.keys()).unwrap();
    other.append(&exec("elsewhere")).unwrap();
    drop(other);

    fs::copy(&other_path, &fixture.path).unwrap();

    assert!(matches!(
        fixture.open(id),
        Err(JournalError::Foreign { .. })
    ));
    assert!(matches!(
        JournalReader::open(&fixture.path, id, fixture.keys()),
        Err(JournalError::Foreign { .. })
    ));
}

#[test]
fn a_deleted_journal_is_reported_missing() {
    let fixture = Fixture::new();
    let id = fixture.with_events(2);

    fs::remove_file(&fixture.path).unwrap();

    assert!(matches!(fixture.open(id), Err(JournalError::Missing(missing)) if missing == id));
    assert!(matches!(
        JournalReader::open(&fixture.path, id, fixture.keys()),
        Err(JournalError::Missing(_))
    ));
}

#[test]
fn a_different_device_key_cannot_vouch_for_the_events() {
    let fixture = Fixture::new();
    let id = fixture.with_events(3);

    let anchors = fixture.keys.anchors(id).unwrap().unwrap();
    let stranger = Arc::new(MemoryKeyStore::with_key(DeviceKey::from_bytes([9; 32])));
    stranger.save_anchors(id, &anchors).unwrap();

    let reader = JournalReader::open(&fixture.path, id, stranger).unwrap();
    assert_eq!(reader.verify().unwrap(), broken(1, Damage::Altered));
}

#[test]
fn events_without_anchors_are_refused() {
    let fixture = Fixture::new();
    let id = fixture.with_events(2);

    fixture.keys.forget(id);

    assert!(matches!(
        fixture.open(id),
        Err(JournalError::AnchorsMissing(_))
    ));
    assert!(matches!(
        fixture.reader(id).verify(),
        Err(JournalError::AnchorsMissing(_))
    ));
}
