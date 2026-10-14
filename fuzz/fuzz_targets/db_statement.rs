#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_db::{Classification, Effect, Engine};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    for engine in [Engine::Postgres, Engine::Mysql, Engine::Sqlite] {
        let classified = Classification::of_sql(engine, text);

        if text.trim().is_empty() {
            assert_eq!(classified.effect, Effect::Privileged);
        }
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let _ = Classification::of_command(&words);
});
