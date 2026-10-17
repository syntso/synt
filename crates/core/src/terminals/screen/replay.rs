use std::collections::VecDeque;

pub(super) struct Replay {
    bytes: VecDeque<u8>,
    limit: usize,
    total: u64,
}

impl Replay {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            bytes: VecDeque::new(),
            limit,
            total: 0,
        }
    }

    pub(super) fn push(&mut self, data: &[u8]) {
        let kept = &data[data.len().saturating_sub(self.limit)..];

        self.bytes.extend(kept);
        self.total += u64::try_from(data.len()).unwrap_or(u64::MAX);

        let excess = self.bytes.len().saturating_sub(self.limit);
        self.bytes.drain(..excess);
    }

    pub(super) fn snapshot(&self) -> Vec<u8> {
        self.bytes.iter().copied().collect()
    }

    pub(super) fn total(&self) -> u64 {
        self.total
    }
}
