mod mark;

use crate::chain::Link;
use crate::seq::Seq;

pub use mark::Mark;

const VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Anchors {
    pub floor: Mark,
    pub head: Mark,
}

impl Anchors {
    pub const ENCODED_LEN: usize = 1 + 2 * Mark::ENCODED_LEN;

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let (&version, marks) = bytes.split_first()?;

        if version != VERSION || marks.len() != 2 * Mark::ENCODED_LEN {
            return None;
        }

        let (floor, head) = marks.split_at(Mark::ENCODED_LEN);

        Some(Self {
            floor: Mark::from_bytes(floor)?,
            head: Mark::from_bytes(head)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::ENCODED_LEN);

        bytes.push(VERSION);
        self.floor.write(&mut bytes);
        self.head.write(&mut bytes);

        bytes
    }

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

#[cfg(test)]
mod tests {
    use super::{Anchors, Mark};
    use crate::chain::Link;
    use crate::seq::Seq;

    fn anchors() -> Anchors {
        Anchors {
            floor: Mark::new(Seq::new(3), Link::from_bytes([1; 32])),
            head: Mark::new(Seq::new(9), Link::from_bytes([2; 32])),
        }
    }

    #[test]
    fn bytes_round_trip() {
        let bytes = anchors().to_bytes();

        assert_eq!(bytes.len(), Anchors::ENCODED_LEN);
        assert_eq!(Anchors::from_bytes(&bytes), Some(anchors()));
    }

    #[test]
    fn bytes_start_with_the_version_and_the_floor() {
        let bytes = anchors().to_bytes();

        assert_eq!(bytes[..9], [1, 0, 0, 0, 0, 0, 0, 0, 3]);
    }

    #[test]
    fn unknown_versions_and_lengths_are_refused() {
        let mut bytes = anchors().to_bytes();
        bytes[0] = 2;

        assert_eq!(Anchors::from_bytes(&bytes), None);
        assert_eq!(Anchors::from_bytes(&bytes[..40]), None);
    }
}
