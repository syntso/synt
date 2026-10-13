//! Positions of events in the journal.

use std::fmt;

/// Position of an event in the journal, starting at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seq(u64);

impl Seq {
    /// Position of the first event in every journal.
    pub const FIRST: Self = Self(1);

    /// Returns the raw number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the position right after this one.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Seq;

    #[test]
    fn starts_at_one() {
        assert_eq!(Seq::FIRST.get(), 1);
    }

    #[test]
    fn next_is_one_more() {
        assert_eq!(Seq::FIRST.next().get(), 2);
    }

    #[test]
    fn displays_with_hash_sign() {
        assert_eq!(Seq::FIRST.to_string(), "#1");
    }
}
