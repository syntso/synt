use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardTarget {
    Pod(String),
    Service(String),
}

impl Display for ForwardTarget {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pod(name) => write!(formatter, "pod/{name}"),
            Self::Service(name) => write!(formatter, "service/{name}"),
        }
    }
}
