use serde::{Serialize, Serializer};
use serde_json::Value;

use super::DetailsError;
use super::encoding::{NULL, decode, encode};
use super::key_material::contains_key_material;
use crate::codec::CodecError;

#[derive(Debug, Clone, PartialEq)]
pub struct Details {
    value: Value,
    bytes: Vec<u8>,
}

impl Details {
    pub const MAX_SIZE: usize = 16 * 1024;

    pub fn empty() -> Self {
        Self {
            value: Value::Null,
            bytes: vec![NULL],
        }
    }

    pub fn from_serialize(value: &impl Serialize) -> Result<Self, DetailsError> {
        let value = serde_json::to_value(value)
            .map_err(|error| DetailsError::Serialize(error.to_string()))?;

        if contains_key_material(&value) {
            return Err(DetailsError::KeyMaterial);
        }

        let mut bytes = Vec::new();
        encode(&value, &mut bytes)?;

        if bytes.len() > Self::MAX_SIZE {
            return Err(DetailsError::TooLarge {
                size: bytes.len(),
                limit: Self::MAX_SIZE,
            });
        }

        Ok(Self { value, bytes })
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn decode(bytes: Vec<u8>) -> Result<Self, CodecError> {
        let value = decode(&bytes)?;
        Ok(Self { value, bytes })
    }
}

impl Serialize for Details {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl Default for Details {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::Details;
    use crate::payload::DetailsError;

    #[derive(Serialize)]
    struct Exec<'a> {
        command: &'a str,
        exit: i32,
    }

    #[test]
    fn builds_from_any_serializable_value() {
        let details = Details::from_serialize(&Exec {
            command: "uptime",
            exit: 0,
        })
        .unwrap();

        assert_eq!(details.value(), &json!({ "command": "uptime", "exit": 0 }));
        assert_eq!(Details::decode(details.bytes().to_vec()), Ok(details));
    }

    #[test]
    fn empty_details_are_null() {
        let empty = Details::empty();

        assert_eq!(empty.value(), &serde_json::Value::Null);
        assert_eq!(Details::decode(empty.bytes().to_vec()), Ok(empty));
    }

    #[test]
    fn refuses_more_than_the_size_limit() {
        let output = "x".repeat(Details::MAX_SIZE);
        let error = Details::from_serialize(&json!({ "output": output })).unwrap_err();

        assert!(matches!(error, DetailsError::TooLarge { .. }));
    }

    #[test]
    fn refuses_key_material() {
        let leak = json!({ "stdout": "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA" });
        assert_eq!(
            Details::from_serialize(&leak),
            Err(DetailsError::KeyMaterial)
        );
    }
}
