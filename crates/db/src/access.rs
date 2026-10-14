#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    ReadWrite,
}

impl Access {
    pub(crate) fn is_read_only(self) -> bool {
        self == Self::ReadOnly
    }
}
