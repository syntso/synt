use std::fmt::{self, Display, Formatter};

use data_encoding::HEXLOWER;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::Serialize;

use crate::ExtensionError;

const RANDOM_BYTES: usize = 12;
const PREFIX: &str = "file-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct FileHandle(String);

impl FileHandle {
    pub(crate) fn fresh() -> Result<Self, ExtensionError> {
        let mut bytes = [0_u8; RANDOM_BYTES];

        getrandom::fill(&mut bytes).map_err(ExtensionError::Random)?;

        Ok(Self(format!("{PREFIX}{}", HEXLOWER.encode(&bytes))))
    }
}

impl From<String> for FileHandle {
    fn from(handle: String) -> Self {
        Self(handle)
    }
}

impl From<FileHandle> for String {
    fn from(handle: FileHandle) -> Self {
        handle.0
    }
}

impl Display for FileHandle {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl ToSql for FileHandle {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for FileHandle {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::FileHandle;

    #[test]
    fn every_handle_is_new() {
        let first = FileHandle::fresh().unwrap();
        let second = FileHandle::fresh().unwrap();

        assert!(first.to_string().starts_with("file-"));
        assert_ne!(first, second);
    }
}
