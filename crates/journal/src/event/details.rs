mod key_material;

use serde::{Serialize, Serializer};
use serde_json::Value;

use self::key_material::KeyMaterial;
use crate::codec::{Decoder, Encoder};
use crate::error::JournalError;

#[derive(Debug, Clone, PartialEq)]
pub struct Details {
    value: Value,
    bytes: Vec<u8>,
}

impl Details {
    pub const MAX_SIZE: usize = 16 * 1024;

    pub fn empty() -> Self {
        let mut bytes = Vec::with_capacity(1);
        Encoder::new(&mut bytes).null();

        Self {
            value: Value::Null,
            bytes,
        }
    }

    pub fn from_serialize(value: &impl Serialize) -> Result<Self, JournalError> {
        let value = serde_json::to_value(value).map_err(JournalError::Unserializable)?;

        if KeyMaterial::found_in(&value) {
            return Err(JournalError::KeyMaterial);
        }

        let mut bytes = Vec::new();
        Encoder::new(&mut bytes).value(&value)?;

        if bytes.len() > Self::MAX_SIZE {
            return Err(JournalError::TooLarge {
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

    pub(crate) fn decode(bytes: Vec<u8>) -> Result<Self, JournalError> {
        let value = Decoder::new(&bytes).value()?;

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
    use crate::error::JournalError;

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
        assert_eq!(Details::decode(details.bytes().to_vec()).unwrap(), details);
    }

    #[test]
    fn empty_details_are_null() {
        let empty = Details::empty();

        assert_eq!(empty.value(), &serde_json::Value::Null);
        assert_eq!(empty.bytes(), [0]);
        assert_eq!(Details::decode(empty.bytes().to_vec()).unwrap(), empty);
    }

    #[test]
    fn refuses_more_than_the_size_limit() {
        let output = "x".repeat(Details::MAX_SIZE);
        let error = Details::from_serialize(&json!({ "output": output })).unwrap_err();

        assert!(matches!(error, JournalError::TooLarge { .. }));
    }

    #[test]
    fn refuses_key_material() {
        let leak = json!({ "stdout": "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA" });

        assert!(matches!(
            Details::from_serialize(&leak),
            Err(JournalError::KeyMaterial)
        ));
    }
}
