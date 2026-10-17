use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use rusqlite::ToSql;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize, Serializer};

use super::VaultKind;
use crate::CoreError;

const LONGEST_NAME: usize = 64;
const REFERENCE_SCHEME: &str = "secret://";

#[derive(Deserialize)]
#[serde(try_from = "String", bound(deserialize = "K: VaultKind"))]
pub struct VaultName<K> {
    text: String,
    kind: PhantomData<K>,
}

impl<K: VaultKind> VaultName<K> {
    pub fn parse(text: &str) -> Result<Self, CoreError> {
        if text.is_empty() || text.len() > LONGEST_NAME {
            return Err(CoreError::VaultNameLength {
                noun: K::NOUN,
                longest: LONGEST_NAME,
            });
        }

        if !text.split('/').all(Self::is_segment) {
            return Err(CoreError::InvalidVaultName {
                noun: K::NOUN,
                text: text.to_owned(),
            });
        }

        Ok(Self::checked(text.to_owned()))
    }

    pub fn from_reference(reference: &str) -> Option<Result<Self, CoreError>> {
        let name = reference.strip_prefix(REFERENCE_SCHEME)?;

        Some(Self::parse(name))
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn reference(&self) -> String {
        format!("{REFERENCE_SCHEME}{}", self.text)
    }

    pub(crate) fn vault_item(&self) -> String {
        format!("{}{}", K::VAULT_PREFIX, self.text.replace('/', "."))
    }

    pub(crate) fn recast<Other: VaultKind>(&self) -> VaultName<Other> {
        VaultName::checked(self.text.clone())
    }

    fn checked(text: String) -> Self {
        Self {
            text,
            kind: PhantomData,
        }
    }

    fn is_segment(segment: &str) -> bool {
        let allowed =
            |character: char| character.is_ascii_alphanumeric() || "-_".contains(character);

        !segment.is_empty() && segment.chars().all(allowed)
    }
}

impl<K: VaultKind> TryFrom<String> for VaultName<K> {
    type Error = CoreError;

    fn try_from(text: String) -> Result<Self, CoreError> {
        Self::parse(&text)
    }
}

impl<K> Clone for VaultName<K> {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            kind: PhantomData,
        }
    }
}

impl<K> PartialEq for VaultName<K> {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl<K> Eq for VaultName<K> {}

impl<K> Hash for VaultName<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl<K> fmt::Debug for VaultName<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.text.fmt(formatter)
    }
}

impl<K> fmt::Display for VaultName<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl<K> Serialize for VaultName<K> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text)
    }
}

impl<K> JsonSchema for VaultName<K> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        String::schema_name()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }
}

impl<K> ToSql for VaultName<K> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.text.to_sql()
    }
}

impl<K: VaultKind> FromSql for VaultName<K> {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?;

        Self::parse(text).map_err(|error| FromSqlError::Other(Box::new(error)))
    }
}
