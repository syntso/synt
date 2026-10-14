#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_db::masked;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    let shown = masked(text);

    assert!(!shown.contains('\''), "a quoted literal survived: {shown}");
    assert!(
        !shown.contains('"'),
        "a double-quoted literal survived: {shown}"
    );
    assert!(!shown.contains("--"), "a line comment survived: {shown}");
    assert!(!shown.contains("/*"), "a block comment survived: {shown}");

    let characters: Vec<char> = text.chars().collect();
    let (first, second) = characters.split_at(characters.len() / 2);
    let first: String = first
        .iter()
        .filter(|character| character.is_alphanumeric() || **character == '\\')
        .collect();
    let second: String = second
        .iter()
        .filter(|character| character.is_ascii_lowercase())
        .collect();

    if second.len() < 4 {
        return;
    }

    let shown = masked(&format!(
        "SELECT * FROM T WHERE A = '{first}' AND B = '{second}'"
    ));
    assert!(
        !shown.contains(&second),
        "the second literal survived: {shown}"
    );
});
