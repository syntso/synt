use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuotaSpent {
    what: &'static str,
    most: u32,
}

impl QuotaSpent {
    pub(super) fn new(what: &'static str, most: u32) -> Self {
        Self { what, most }
    }
}

impl Display for QuotaSpent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "more than {} {} in one call",
            self.most, self.what
        )
    }
}

impl Error for QuotaSpent {}
