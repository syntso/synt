use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize, Serializer};

use super::VaultKind;
use crate::PlatformError;

const LONGEST_NAME: usize = 64;
const REFERENCE_SCHEME: &str = "secret://";

#[derive(Deserialize)]
#[serde(try_from = "String", bound(deserialize = "K: VaultKind"))]
pub struct VaultName<K> {
    text: String,
    kind: PhantomData<K>,
}

impl<K: VaultKind> VaultName<K> {
    pub const LONGEST: usize = LONGEST_NAME;

    pub fn parse(text: &str) -> Result<Self, PlatformError> {
        if text.is_empty() || text.len() > LONGEST_NAME {
            return Err(PlatformError::VaultNameLength {
                noun: K::NOUN,
                longest: LONGEST_NAME,
            });
        }

        if !text.split('/').all(Self::is_segment) {
            return Err(PlatformError::InvalidVaultName {
                noun: K::NOUN,
                text: text.to_owned(),
            });
        }

        Ok(Self::checked(text.to_owned()))
    }

    pub fn from_reference(reference: &str) -> Option<Result<Self, PlatformError>> {
        let name = reference.strip_prefix(REFERENCE_SCHEME)?;

        Some(Self::parse(name))
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn reference(&self) -> String {
        format!("{REFERENCE_SCHEME}{}", self.text)
    }

    pub fn vault_item(&self) -> String {
        format!("{}{}", K::VAULT_PREFIX, self.text.replace('/', "."))
    }

    pub fn recast<Other: VaultKind>(&self) -> VaultName<Other> {
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
    type Error = PlatformError;

    fn try_from(text: String) -> Result<Self, PlatformError> {
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

impl<K> PartialOrd for VaultName<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K> Ord for VaultName<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.text.cmp(&other.text)
    }
}

impl<K> Hash for VaultName<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl<K> Borrow<str> for VaultName<K> {
    fn borrow(&self) -> &str {
        &self.text
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
