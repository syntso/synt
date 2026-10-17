#![no_main]

use libfuzzer_sys::fuzz_target;
use synt_platform::fuzzing::answer_cursor_question;

const QUESTION: &[u8] = b"\x1b[6n";

fuzz_target!(|data: &[u8]| {
    let (whole, split) = synt_fuzz::pieces(data);
    let (shown, answers) = answer_cursor_question(&split);

    let expected = match whole
        .windows(QUESTION.len())
        .position(|window| window == QUESTION)
    {
        Some(at) => [&whole[..at], &whole[at + QUESTION.len()..]].concat(),
        None => whole.to_vec(),
    };
    let asked = usize::from(expected.len() != whole.len());

    assert_eq!(shown, expected);
    assert_eq!(answers, asked);
});
