use std::fmt::{self, Display, Formatter};

use data_encoding::HEXLOWER;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256 as Hasher};

use crate::ExtensionError;

const HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct Sha256(String);

impl Sha256 {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self(HEXLOWER.encode(&Hasher::digest(bytes)))
    }

    pub(crate) fn recorded(text: &str) -> Self {
        Self(text.to_owned())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sha256 {
    type Error = ExtensionError;

    fn try_from(text: String) -> Result<Self, ExtensionError> {
        let hex = text
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character));

        if text.len() != HEX_LENGTH || !hex {
            return Err(ExtensionError::NotSha256(text));
        }

        Ok(Self(text))
    }
}

impl From<Sha256> for String {
    fn from(digest: Sha256) -> Self {
        digest.0
    }
}

impl Display for Sha256 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl ToSql for Sha256 {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for Sha256 {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256;

    #[test]
    fn a_digest_is_lowercase_hex_and_parses_back() {
        let digest = Sha256::of(b"hello");

        assert_eq!(digest.as_str().len(), 64);
        assert_eq!(
            Sha256::try_from(digest.as_str().to_owned()).unwrap(),
            digest
        );
    }

    #[test]
    fn other_text_is_not_a_digest() {
        for text in ["", "abc", &"G".repeat(64), &"A".repeat(64)] {
            assert!(Sha256::try_from(text.to_owned()).is_err(), "{text}");
        }
    }
}
