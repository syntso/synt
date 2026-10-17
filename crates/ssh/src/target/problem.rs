use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProblem {
    MissingBracket,
    TextAfterBracket,
    BadPort,
    EmptyHost,
    EmptyUser,
    Wildcard,
    NoUser,
}

impl Display for TargetProblem {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::MissingBracket => "missing ']'",
            Self::TextAfterBracket => "unexpected text after ']'",
            Self::BadPort => "the port must be a number from 1 to 65535",
            Self::EmptyHost => "the host is empty",
            Self::EmptyUser => "the user is empty",
            Self::Wildcard => "a target names one host, so it cannot hold * or ?",
            Self::NoUser => "no user given and none found in the environment",
        };

        formatter.write_str(reason)
    }
}
