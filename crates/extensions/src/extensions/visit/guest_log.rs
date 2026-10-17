use std::mem;

use crate::interface::Level;

const MOST_LINES: usize = 200;
const MOST_BYTES: usize = 64 * 1024;
const LONGEST_LINE: usize = 4096;

pub(super) struct GuestLog {
    extension: String,
    lines: usize,
    bytes: usize,
    cut: bool,
}

impl GuestLog {
    pub(super) fn new(extension: &str) -> Self {
        Self {
            extension: extension.to_owned(),
            lines: 0,
            bytes: 0,
            cut: false,
        }
    }

    pub(super) fn write(&mut self, level: Level, message: &str) {
        let shown = Self::shown(message);

        if !self.admit(shown.len()) {
            self.note_cut();
            return;
        }

        eprintln!(
            "synt-core: extension {}: {}: {shown}",
            self.extension,
            level.name()
        );
    }

    fn shown(message: &str) -> String {
        message
            .chars()
            .take(LONGEST_LINE)
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect()
    }

    fn admit(&mut self, length: usize) -> bool {
        let fits = self.lines < MOST_LINES && self.bytes + length <= MOST_BYTES;

        if fits {
            self.lines += 1;
            self.bytes += length;
        }

        fits
    }

    fn note_cut(&mut self) {
        if !mem::replace(&mut self.cut, true) {
            eprintln!(
                "synt-core: extension {}: further log lines of this call are dropped",
                self.extension
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GuestLog, MOST_BYTES, MOST_LINES};

    #[test]
    fn stops_after_the_most_lines() {
        let mut log = GuestLog::new("t");

        for _ in 0..MOST_LINES {
            assert!(log.admit(1));
        }

        assert!(!log.admit(1));
    }

    #[test]
    fn stops_after_the_most_bytes() {
        let mut log = GuestLog::new("t");

        assert!(log.admit(MOST_BYTES - 10));
        assert!(!log.admit(11));
        assert!(log.admit(10));
    }

    #[test]
    fn control_characters_become_spaces() {
        assert_eq!(GuestLog::shown("a\nb\tc"), "a b c");
    }
}
