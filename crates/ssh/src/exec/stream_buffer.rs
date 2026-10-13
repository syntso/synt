pub(super) struct StreamBuffer {
    bytes: Vec<u8>,
    limit: usize,
    truncated: bool,
}

impl StreamBuffer {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            truncated: false,
        }
    }

    pub(super) fn push(&mut self, data: &[u8]) {
        let room = self.limit.saturating_sub(self.bytes.len());
        let kept = data.len().min(room);

        self.bytes.extend_from_slice(&data[..kept]);
        self.truncated |= kept < data.len();
    }

    pub(super) fn truncated(&self) -> bool {
        self.truncated
    }

    pub(super) fn into_text(self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}
