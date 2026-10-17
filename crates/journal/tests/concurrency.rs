use std::collections::BTreeSet;
use std::thread;

use synt_journal::{JournalError, JournalWriter, Seq};
use synt_testkit::TestError;
use synt_testkit::journal::{Fixture, exec};

#[test]
fn concurrent_appends_get_unique_consecutive_seqs() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let writer = JournalWriter::spawn(fixture.create()?).unwrap();

    let seqs: BTreeSet<Seq> = thread::scope(|scope| {
        let workers: Vec<_> = (0..16)
            .map(|worker| {
                let writer = writer.clone();

                scope.spawn(move || {
                    (0..50)
                        .map(|n| {
                            writer
                                .append(exec(&format!("worker {worker} step {n}")).unwrap())
                                .unwrap()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect()
    });

    let expected: BTreeSet<Seq> = (1..=800).map(Seq::new).collect();
    assert_eq!(seqs, expected);
    assert!(writer.reader().unwrap().verify().unwrap().is_intact());

    Ok(())
}

#[test]
fn each_caller_sees_its_own_events_in_order() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let writer = JournalWriter::spawn(fixture.create()?).unwrap();

    let seqs: Vec<Seq> = (0..20)
        .map(|n| writer.append(exec(&format!("{n}")).unwrap()).unwrap())
        .collect();

    assert!(seqs.windows(2).all(|pair| pair[0] < pair[1]));

    Ok(())
}

#[tokio::test]
async fn async_appends_share_the_chain_with_blocking_ones() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let writer = JournalWriter::spawn(fixture.create()?).unwrap();

    let first = writer.append_async(exec("first")?).await.unwrap();
    let second = writer.append_async(exec("second")?).await.unwrap();

    assert_eq!((first, second), (Seq::new(1), Seq::new(2)));
    assert!(writer.reader().unwrap().verify().unwrap().is_intact());

    Ok(())
}

#[test]
fn a_second_writer_is_refused() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let journal = fixture.create()?;

    assert!(matches!(
        fixture.open(journal.id()),
        Err(JournalError::Locked)
    ));

    Ok(())
}

#[test]
fn the_journal_is_released_when_the_last_writer_handle_drops() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let writer = JournalWriter::spawn(fixture.create()?).unwrap();
    let id = writer.id();

    let clone = writer.clone();
    clone.append(exec("from a clone")?).unwrap();
    drop(writer);
    drop(clone);

    let journal = fixture.open(id).unwrap();
    assert_eq!(journal.anchors().head.seq, Seq::new(1));

    Ok(())
}

#[test]
fn readers_work_while_the_writer_is_busy() -> Result<(), TestError> {
    let fixture = Fixture::new()?;
    let writer = JournalWriter::spawn(fixture.create()?).unwrap();
    let reader = writer.reader().unwrap();

    for n in 0..10 {
        writer.append(exec(&format!("{n}"))?).unwrap();
        assert!(reader.verify().unwrap().is_intact());
    }

    Ok(())
}
