#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_sync::fuzzing::{document_kind, open_sealed};

fuzz_target!(|data: &[u8]| {
    assert!(!open_sealed(data));

    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Some(kind) = document_kind(text) {
        assert!(!text.contains(".."));
        assert!(!text.starts_with('/'));
        assert!(kind.starts_with("core.") || kind.starts_with("ext."));
    }
});
