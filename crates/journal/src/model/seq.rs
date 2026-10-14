use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Seq(u64);

impl Seq {
    pub const ORIGIN: Self = Self(0);
    pub const FIRST: Self = Self(1);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn to_sql(self) -> i64 {
        i64::try_from(self.0).unwrap_or(i64::MAX)
    }

    pub(crate) fn from_sql(value: i64) -> Option<Self> {
        u64::try_from(value).ok().map(Self)
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
    fn counts_from_one_after_the_origin() {
        assert_eq!(Seq::ORIGIN.next(), Seq::FIRST);
        assert_eq!(Seq::FIRST.next().get(), 2);
    }

    #[test]
    fn displays_with_a_hash_sign() {
        assert_eq!(Seq::new(42).to_string(), "#42");
    }

    #[test]
    fn refuses_negative_sql_values() {
        assert_eq!(Seq::from_sql(-1), None);
        assert_eq!(Seq::from_sql(5), Some(Seq::new(5)));
    }
}
