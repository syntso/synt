use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unanswered {
    RequesterGone,
    Unsendable,
    NoOneAvailable,
    TimedOut,
}

impl Display for Unanswered {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::RequesterGone => "the requester disconnected",
            Self::Unsendable => "the approval request cannot be sent",
            Self::NoOneAvailable => "no one is available to approve",
            Self::TimedOut => "approval timed out",
        };

        formatter.write_str(reason)
    }
}
