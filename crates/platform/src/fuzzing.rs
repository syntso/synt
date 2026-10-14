use tokio::sync::mpsc;

use crate::pty::CursorQuestion;

pub fn answer_cursor_question(chunks: &[&[u8]]) -> (Vec<u8>, usize) {
    let (input, mut typed) = mpsc::channel(1);
    let mut question = CursorQuestion::expected(input);
    let mut shown = Vec::new();

    for chunk in chunks {
        shown.extend(question.screen(chunk));
    }
    shown.extend(question.rest());

    let mut answers = 0;
    while typed.try_recv().is_ok() {
        answers += 1;
    }

    (shown, answers)
}
