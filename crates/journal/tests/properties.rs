use proptest::prelude::*;
use serde_json::{Value, json};
use synt_journal::{Actor, Damage, Details, EventKind, Name, NewEvent, Seq, Verification};
use synt_testkit::journal::{Fixture, action};

fn actor() -> impl Strategy<Value = Actor> {
    let name = "[a-z][a-z0-9-]{0,15}".prop_filter_map("a valid name", |text| Name::new(text).ok());

    prop_oneof![
        Just(Actor::Person),
        Just(Actor::Core),
        name.clone().prop_map(Actor::Agent),
        name.clone().prop_map(Actor::Runbook),
        name.prop_map(Actor::Plugin),
    ]
}

fn kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        Just(EventKind::Requested),
        Just(EventKind::Approved),
        Just(EventKind::Denied),
        Just(EventKind::Started),
        Just(EventKind::Finished),
        Just(EventKind::Failed),
        Just(EventKind::Note),
    ]
}

fn details() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        "[ -~]{0,24}".prop_map(Value::from),
    ];

    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::from),
            prop::collection::btree_map("[a-z]{1,6}", inner, 0..4)
                .prop_map(|entries| json!(entries)),
        ]
    })
}

fn event() -> impl Strategy<Value = NewEvent> {
    (actor(), kind(), "[a-z]{1,8}\\.[a-z]{1,8}", details()).prop_filter_map(
        "a valid event",
        |(actor, kind, id, value)| {
            let action = action(&id).ok()?;
            let details = Details::from_serialize(&value).ok()?;

            Some(NewEvent::new(actor, kind, action).details(details))
        },
    )
}

const COLUMNS: [&str; 6] = ["at", "actor", "kind", "action", "details", "link"];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn any_sequence_verifies_and_reads_back(events in prop::collection::vec(event(), 1..24)) {
        let fixture = Fixture::new().unwrap();
        let mut journal = fixture.create().unwrap();

        journal.append_all(&events).unwrap();
        let reader = journal.reader().unwrap();

        prop_assert_eq!(reader.verify().unwrap(), Verification::Intact { checked: events.len() as u64 });

        for (index, expected) in events.iter().enumerate() {
            let stored = reader.get(Seq::new(index as u64 + 1)).unwrap().unwrap();

            prop_assert_eq!(stored.actor(), expected.actor());
            prop_assert_eq!(stored.kind(), expected.kind());
        }
    }

    #[test]
    fn any_change_to_any_stored_field_is_found_at_that_event(
        events in prop::collection::vec(event(), 1..12),
        target in any::<prop::sample::Index>(),
        column in 0..COLUMNS.len(),
    ) {
        let fixture = Fixture::new().unwrap();
        let mut journal = fixture.create().unwrap();
        let id = journal.id();

        journal.append_all(&events).unwrap();
        drop(journal);

        let seq = target.index(events.len()) as u64 + 1;
        let column = COLUMNS[column];
        let change = match column {
            "at" => "at + 1".to_owned(),
            "kind" => "kind % 7 + 1".to_owned(),
            "action" => "action || 'x'".to_owned(),
            _ => format!("CAST({column} || X'00' AS BLOB)"),
        };

        fixture.raw().unwrap().execute(&format!("UPDATE events SET {column} = {change} WHERE seq = ?1"), [i64::try_from(seq).unwrap()]).unwrap();

        prop_assert_eq!(
            fixture.reader(id).unwrap().verify().unwrap(),
            Verification::Broken { at: Seq::new(seq), damage: Damage::Altered }
        );
    }
}
