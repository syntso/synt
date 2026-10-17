use tokio::sync::mpsc;

const QUESTION: &[u8] = b"\x1b[6n";
const ANSWER: &[u8] = b"\x1b[1;1R";

pub(crate) struct CursorQuestion {
    held: Vec<u8>,
    answer_to: Option<mpsc::Sender<Vec<u8>>>,
}

impl CursorQuestion {
    pub(crate) fn expected(input: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            held: Vec::new(),
            answer_to: Some(input),
        }
    }

    pub(crate) fn not_expected() -> Self {
        Self {
            held: Vec::new(),
            answer_to: None,
        }
    }

    pub(crate) fn screen(&mut self, chunk: &[u8]) -> Vec<u8> {
        let Some(input) = self.answer_to.take() else {
            return chunk.to_vec();
        };

        let mut shown = std::mem::take(&mut self.held);
        shown.extend_from_slice(chunk);

        if let Some(at) = Self::find(&shown) {
            shown.drain(at..at + QUESTION.len());
            let _ = input.blocking_send(ANSWER.to_vec());
            return shown;
        }

        let kept = shown.len() - Self::partial_question_at_end(&shown);
        self.held = shown.split_off(kept);
        self.answer_to = Some(input);

        shown
    }

    pub(crate) fn rest(self) -> Vec<u8> {
        self.held
    }

    fn find(shown: &[u8]) -> Option<usize> {
        shown
            .windows(QUESTION.len())
            .position(|window| window == QUESTION)
    }

    fn partial_question_at_end(shown: &[u8]) -> usize {
        (1..QUESTION.len())
            .rev()
            .find(|&length| shown.ends_with(&QUESTION[..length]))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answering() -> (CursorQuestion, mpsc::Receiver<Vec<u8>>) {
        let (input, typed) = mpsc::channel(4);

        (CursorQuestion::expected(input), typed)
    }

    #[test]
    fn the_question_is_answered_once_and_never_shown() {
        let (mut question, mut typed) = answering();

        let shown = question.screen(b"\x1b[?25l\x1b[6nhello");
        let later = question.screen(b"\x1b[6n");

        assert_eq!(shown, b"\x1b[?25lhello");
        assert_eq!(later, b"\x1b[6n");
        assert_eq!(typed.try_recv().unwrap(), ANSWER);
        assert!(typed.try_recv().is_err());
    }

    #[test]
    fn a_question_split_across_chunks_is_still_answered() {
        let (mut question, mut typed) = answering();

        let first = question.screen(b"start\x1b[");
        let second = question.screen(b"6nend");

        assert_eq!(first, b"start");
        assert_eq!(second, b"end");
        assert_eq!(typed.try_recv().unwrap(), ANSWER);
    }

    #[test]
    fn output_that_only_looks_like_the_question_is_shown_whole() {
        let (mut question, mut typed) = answering();

        let first = question.screen(b"text\x1b");
        let second = question.screen(b"[1mbold");

        assert_eq!([first, second].concat(), b"text\x1b[1mbold");
        assert!(typed.try_recv().is_err());
    }

    #[test]
    fn a_held_beginning_is_given_back_at_the_end() {
        let (mut question, _typed) = answering();

        let shown = question.screen(b"bye\x1b[6");

        assert_eq!(shown, b"bye");
        assert_eq!(question.rest(), b"\x1b[6");
    }

    #[test]
    fn a_terminal_that_asks_nothing_passes_output_through() {
        let mut question = CursorQuestion::not_expected();

        let shown = question.screen(b"\x1b[6n");

        assert_eq!(shown, b"\x1b[6n");
        assert!(question.rest().is_empty());
    }
}
