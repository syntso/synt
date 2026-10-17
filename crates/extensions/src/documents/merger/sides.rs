#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeSides<'a> {
    base: Option<&'a [u8]>,
    ours: &'a [u8],
    theirs: &'a [u8],
}

impl<'a> MergeSides<'a> {
    pub fn new(base: Option<&'a [u8]>, ours: &'a [u8], theirs: &'a [u8]) -> Self {
        Self { base, ours, theirs }
    }

    pub(crate) fn base(&self) -> Option<&'a [u8]> {
        self.base
    }

    pub(crate) fn ours(&self) -> &'a [u8] {
        self.ours
    }

    pub(crate) fn theirs(&self) -> &'a [u8] {
        self.theirs
    }
}
