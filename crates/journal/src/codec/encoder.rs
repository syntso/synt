pub(crate) struct Encoder<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.out.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn i64(&mut self, value: i64) {
        self.out.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn len(&mut self, len: usize) {
        let len = u32::try_from(len).unwrap_or(u32::MAX);
        self.out.extend_from_slice(&len.to_be_bytes());
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.len(value.len());
        self.out.extend_from_slice(value);
    }

    pub(crate) fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub(crate) fn optional<T>(&mut self, value: Option<T>, write: impl FnOnce(&mut Self, T)) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                write(self, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn prefixes_bytes_with_their_length() {
        let mut out = Vec::new();
        Encoder::new(&mut out).str("ab");

        assert_eq!(out, [0, 0, 0, 2, b'a', b'b']);
    }

    #[test]
    fn marks_absent_and_present_options() {
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out);

        encoder.optional(None::<u8>, Encoder::u8);
        encoder.optional(Some(7), Encoder::u8);

        assert_eq!(out, [0, 1, 7]);
    }
}
