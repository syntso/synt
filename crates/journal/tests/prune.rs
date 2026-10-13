#![allow(clippy::unwrap_used)]

mod common;

use common::{Fixture, exec};
use synt_journal::{Damage, JournalError, Pruned, Seq, Timestamp, Verification};

const HOUR: i64 = 3_600_000;

#[test]
fn pruning_removes_old_events_and_keeps_the_chain_verifiable() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();

    for n in 0..10 {
        journal.append(&exec(&format!("hour {n}"))).unwrap();
        fixture.clock.advance(HOUR);
    }

    let cutoff = Timestamp::from_millis(1_700_000_000_000 + 4 * HOUR);
    let pruned = journal.prune(cutoff).unwrap();

    assert_eq!(
        pruned,
        Pruned {
            removed: 4,
            floor: Seq::new(4)
        }
    );
    assert_eq!(journal.anchors().head.seq, Seq::new(10));

    let reader = journal.reader().unwrap();
    assert_eq!(reader.get(Seq::new(4)).unwrap(), None);
    assert!(reader.get(Seq::new(5)).unwrap().is_some());
    assert_eq!(
        reader.verify().unwrap(),
        Verification::Intact { checked: 6 }
    );
}

#[test]
fn pruning_never_moves_the_head() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();

    journal
        .append_all(&[exec("a"), exec("b"), exec("c")])
        .unwrap();
    let head = journal.anchors().head;

    fixture.clock.advance(HOUR);
    let pruned = journal.prune(Timestamp::from_millis(i64::MAX)).unwrap();

    assert_eq!(pruned.removed, 3);
    assert_eq!(journal.anchors().head, head);
    assert_eq!(
        journal.reader().unwrap().verify().unwrap(),
        Verification::Intact { checked: 0 }
    );

    let next = journal.append(&exec("d")).unwrap();
    assert_eq!(next, Seq::new(4));
    assert!(journal.reader().unwrap().verify().unwrap().is_intact());
}

#[test]
fn pruning_with_nothing_old_enough_does_nothing() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    journal.append(&exec("fresh")).unwrap();

    let pruned = journal.prune(Timestamp::EPOCH).unwrap();
    assert_eq!(
        pruned,
        Pruned {
            removed: 0,
            floor: Seq::ORIGIN
        }
    );
}

#[test]
fn pruning_refuses_to_bury_tampered_history() {
    let fixture = Fixture::new();
    let id = fixture.with_events(5);

    fixture
        .raw()
        .execute_batch("UPDATE events SET action = 'forged' WHERE seq = 2")
        .unwrap();

    let mut journal = fixture.open(id).unwrap();
    let refused = journal.prune(Timestamp::from_millis(i64::MAX));

    assert!(matches!(
        refused,
        Err(JournalError::Tampered(Verification::Broken { at, damage: Damage::Altered })) if at == Seq::new(2)
    ));
    assert!(
        journal
            .reader()
            .unwrap()
            .get(Seq::new(1))
            .unwrap()
            .is_some()
    );
}
