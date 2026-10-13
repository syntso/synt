#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use common::{Fixture, FlakyKeyStore, exec};
use synt_journal::{JournalBuilder, JournalError, KeyStore, Seq};

#[test]
fn events_written_before_a_failed_anchor_save_are_recovered_on_open() {
    let fixture = Fixture::new();
    let flaky = Arc::new(FlakyKeyStore::new(fixture.keys()));

    let mut journal = JournalBuilder::new(flaky.clone())
        .create(&fixture.path)
        .unwrap();
    let id = journal.id();
    journal.append(&exec("one")).unwrap();

    flaky.fail_saves(true);
    let failed = journal.append(&exec("two"));
    drop(journal);

    assert!(matches!(failed, Err(JournalError::KeyStore(_))));
    assert_eq!(
        fixture.keys.anchors(id).unwrap().unwrap().head.seq,
        Seq::new(1)
    );

    let journal = fixture.open(id).unwrap();

    assert_eq!(journal.anchors().head.seq, Seq::new(2));
    assert!(journal.reader().unwrap().verify().unwrap().is_intact());
}

#[test]
fn a_journal_keeps_working_after_the_key_store_comes_back() {
    let fixture = Fixture::new();
    let flaky = Arc::new(FlakyKeyStore::new(fixture.keys()));

    let mut journal = JournalBuilder::new(flaky.clone())
        .create(&fixture.path)
        .unwrap();
    let id = journal.id();

    flaky.fail_saves(true);
    assert!(journal.append(&exec("lost acknowledgement")).is_err());

    flaky.fail_saves(false);
    let next = journal.append(&exec("back to normal")).unwrap();

    assert_eq!(next, Seq::new(2));
    assert_eq!(
        fixture.keys.anchors(id).unwrap().unwrap().head.seq,
        Seq::new(2)
    );
    assert!(journal.reader().unwrap().verify().unwrap().is_intact());
}

#[test]
fn a_journal_cannot_be_created_twice() {
    let fixture = Fixture::new();
    fixture.with_events(1);

    let again = JournalBuilder::new(fixture.keys()).create(&fixture.path);
    assert!(matches!(again, Err(JournalError::AlreadyExists(_))));
}

#[test]
fn opening_a_journal_that_never_existed_is_not_found() {
    let fixture = Fixture::new();
    let id = synt_journal::JournalId::from_bytes([3; 16]);

    assert!(matches!(fixture.open(id), Err(JournalError::NotFound(_))));
}
