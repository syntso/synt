use std::fmt::{self, Display, Formatter};

use serde::{Serialize, Serializer};

const UNPARSABLE: &str = "unparsable";
const SEVERAL: &str = "several statements";
const EMPTY: &str = "empty";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Named(String),
    Unparsable,
    Several,
    Empty,
}

impl StatementKind {
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    pub fn is(&self, name: &str) -> bool {
        matches!(self, Self::Named(own) if own == name)
    }

    pub fn is_one_statement(&self) -> bool {
        matches!(self, Self::Named(_))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Named(name) => name,
            Self::Unparsable => UNPARSABLE,
            Self::Several => SEVERAL,
            Self::Empty => EMPTY,
        }
    }
}

impl Display for StatementKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

impl Serialize for StatementKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StatementKind;

    #[test]
    fn kinds_are_written_as_plain_words() {
        assert_eq!(json!(StatementKind::named("select")), json!("select"));
        assert_eq!(json!(StatementKind::Several), json!("several statements"));
        assert_eq!(json!(StatementKind::Unparsable), json!("unparsable"));
        assert_eq!(json!(StatementKind::Empty), json!("empty"));
    }

    #[test]
    fn only_a_named_kind_is_one_statement() {
        assert!(StatementKind::named("drop").is_one_statement());
        assert!(!StatementKind::Several.is_one_statement());
        assert!(StatementKind::named("explain").is("explain"));
    }
}
