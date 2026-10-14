use std::mem;

const MOST_LINES: usize = 200;
const MOST_BYTES: usize = 64 * 1024;

pub(super) struct LogAllowance {
    lines: usize,
    bytes: usize,
    cut: bool,
}

impl LogAllowance {
    pub(super) fn new() -> Self {
        Self {
            lines: 0,
            bytes: 0,
            cut: false,
        }
    }

    pub(super) fn admit(&mut self, length: usize) -> bool {
        let fits = self.lines < MOST_LINES && self.bytes + length <= MOST_BYTES;

        if fits {
            self.lines += 1;
            self.bytes += length;
        }

        fits
    }

    pub(super) fn cut_once(&mut self) -> bool {
        !mem::replace(&mut self.cut, true)
    }
}

#[cfg(test)]
mod tests {
    use super::{LogAllowance, MOST_BYTES, MOST_LINES};

    #[test]
    fn stops_after_the_most_lines() {
        let mut allowance = LogAllowance::new();

        for _ in 0..MOST_LINES {
            assert!(allowance.admit(1));
        }

        assert!(!allowance.admit(1));
        assert!(allowance.cut_once());
        assert!(!allowance.cut_once());
    }

    #[test]
    fn stops_after_the_most_bytes() {
        let mut allowance = LogAllowance::new();

        assert!(allowance.admit(MOST_BYTES - 10));
        assert!(!allowance.admit(11));
        assert!(allowance.admit(10));
    }
}
