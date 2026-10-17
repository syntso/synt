use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginStatus {
    Registered {
        place: String,
        program: Option<PathBuf>,
    },
    NotRegistered,
}

impl Display for LoginStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registered {
                place,
                program: Some(program),
            } => write!(
                formatter,
                "starts at login: {place}, runs {}",
                program.display()
            ),
            Self::Registered {
                place,
                program: None,
            } => write!(formatter, "starts at login: {place}"),
            Self::NotRegistered => formatter.write_str("does not start at login"),
        }
    }
}
