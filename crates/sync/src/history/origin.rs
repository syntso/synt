#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    Pulled,
    Local,
}

impl Origin {
    pub(super) fn is_pushed(self) -> bool {
        self == Self::Pulled
    }
}
