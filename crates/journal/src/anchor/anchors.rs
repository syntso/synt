use super::Mark;
use crate::chain::Link;
use crate::model::Seq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Anchors {
    pub floor: Mark,
    pub head: Mark,
}

impl Anchors {
    pub(crate) fn genesis(link: Link) -> Self {
        let origin = Mark::new(Seq::ORIGIN, link);
        Self {
            floor: origin,
            head: origin,
        }
    }

    #[must_use]
    pub(crate) fn with_head(self, head: Mark) -> Self {
        Self { head, ..self }
    }

    #[must_use]
    pub(crate) fn with_floor(self, floor: Mark) -> Self {
        Self { floor, ..self }
    }
}
