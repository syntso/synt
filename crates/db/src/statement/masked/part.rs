#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    Shown,
    Literal,
    Comment,
}

impl Part {
    pub(super) fn stricter(self, other: Self) -> Self {
        match (self, other) {
            (Self::Literal, _) | (_, Self::Literal) => Self::Literal,
            (Self::Comment, _) | (_, Self::Comment) => Self::Comment,
            (Self::Shown, Self::Shown) => Self::Shown,
        }
    }
}
