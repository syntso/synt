use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::Serialize;

use crate::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum TableKind {
    #[serde(rename = "table")]
    Table,

    #[serde(rename = "view")]
    View,

    #[serde(rename = "materialized view")]
    MaterializedView,

    #[serde(rename = "foreign table")]
    ForeignTable,
}

impl TableKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::View => "view",
            Self::MaterializedView => "materialized view",
            Self::ForeignTable => "foreign table",
        }
    }
}

impl FromStr for TableKind {
    type Err = DbError;

    fn from_str(text: &str) -> Result<Self, DbError> {
        [
            Self::Table,
            Self::View,
            Self::MaterializedView,
            Self::ForeignTable,
        ]
        .into_iter()
        .find(|kind| kind.name() == text)
        .ok_or_else(DbError::unexpected_catalog_row)
    }
}

impl Display for TableKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::TableKind;

    #[test]
    fn kinds_read_and_write_the_words_the_catalogs_use() {
        for word in ["table", "view", "materialized view", "foreign table"] {
            let kind: TableKind = word.parse().expect("known kind");

            assert_eq!(json!(kind), json!(word));
        }
    }

    #[test]
    fn an_unknown_kind_is_an_unexpected_row() {
        assert!("sequence".parse::<TableKind>().is_err());
    }
}
