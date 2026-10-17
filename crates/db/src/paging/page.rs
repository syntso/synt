#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    rows: usize,
    bytes: usize,
    offset: u64,
}

impl Page {
    pub fn new(rows: usize, bytes: usize) -> Self {
        Self {
            rows: rows.max(1),
            bytes: bytes.max(1),
            offset: 0,
        }
    }

    #[must_use]
    pub fn starting_at(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }
}
