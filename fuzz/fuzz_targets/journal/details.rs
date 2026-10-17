#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_journal::Details;
use synt_journal::fuzzing::{decode_details, details_bytes};

fuzz_target!(|data: &[u8]| {
    let Some(details) = decode_details(data) else {
        return;
    };

    assert_eq!(details_bytes(&details), data);

    if let Ok(rebuilt) = Details::from_serialize(details.value()) {
        assert_eq!(details_bytes(&rebuilt), data);
        assert_eq!(rebuilt, details);
    }

    let json = serde_json::to_vec(&details).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(&value, details.value());
});
