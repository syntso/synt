pub(crate) struct Canonical {
    bytes: Vec<u8>,
}

impl Canonical {
    pub(crate) fn new(label: &str) -> Self {
        let canonical = Self { bytes: Vec::new() };

        canonical.text(label)
    }

    pub(crate) fn text(self, text: &str) -> Self {
        self.bytes(text.as_bytes())
    }

    pub(crate) fn bytes(mut self, bytes: &[u8]) -> Self {
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);

        self.bytes.extend_from_slice(&length.to_be_bytes());
        self.bytes.extend_from_slice(bytes);
        self
    }

    pub(crate) fn number(mut self, number: u64) -> Self {
        self.bytes.extend_from_slice(&number.to_be_bytes());
        self
    }

    pub(crate) fn optional(self, bytes: Option<&[u8]>) -> Self {
        match bytes {
            Some(bytes) => self.number(1).bytes(bytes),
            None => self.number(0),
        }
    }

    pub(crate) fn texts<'a>(self, texts: impl ExactSizeIterator<Item = &'a str>) -> Self {
        let count = u64::try_from(texts.len()).unwrap_or(u64::MAX);

        texts.fold(self.number(count), Self::text)
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::Canonical;

    #[test]
    fn moving_bytes_between_fields_changes_the_result() {
        let one = Canonical::new("x").text("ab").text("c").finish();
        let other = Canonical::new("x").text("a").text("bc").finish();

        assert_ne!(one, other);
    }

    #[test]
    fn a_missing_value_differs_from_an_empty_one() {
        let missing = Canonical::new("x").optional(None).finish();
        let empty = Canonical::new("x").optional(Some(b"")).finish();

        assert_ne!(missing, empty);
    }

    #[test]
    fn lists_are_counted() {
        let two = Canonical::new("x").texts(["a", "b"].into_iter()).finish();
        let joined = Canonical::new("x").texts(["ab"].into_iter()).finish();

        assert_ne!(two, joined);
    }
}
