#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_db::masked;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    let shown = masked(text);

    assert!(!shown.contains('\''), "a quoted literal survived: {shown}");
    assert!(!shown.contains('"'), "a double-quoted literal survived: {shown}");
    assert!(!shown.contains("--"), "a line comment survived: {shown}");
    assert!(!shown.contains("/*"), "a block comment survived: {shown}");
});
