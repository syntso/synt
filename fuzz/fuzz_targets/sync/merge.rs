#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_sync::fuzzing::merge;

const SEPARATOR: u8 = 0xff;

fuzz_target!(|data: &[u8]| {
    let mut parts = data.splitn(3, |byte| *byte == SEPARATOR);
    let (Some(base), Some(older), Some(newer)) = (parts.next(), parts.next(), parts.next()) else {
        return;
    };

    let merged = merge(Some(base), older, newer);
    assert_eq!(merged, merge(Some(base), older, newer));

    let untouched = merge(Some(base), base, newer);
    assert_eq!(untouched.body.as_deref(), Some(newer));

    let parses = |bytes: &[u8]| {
        std::str::from_utf8(bytes)
            .ok()
            .is_some_and(|text| text.parse::<toml::Table>().is_ok())
    };
    if parses(older) && parses(newer) {
        assert!(parses(&merged.body.unwrap()));
    }
});
