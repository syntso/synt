use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

const VERSION: &str = "2.0";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct JsonRpcVersion;

impl Serialize for JsonRpcVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(VERSION)
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let version = String::deserialize(deserializer)?;

        if version != VERSION {
            return Err(de::Error::invalid_value(
                de::Unexpected::Str(&version),
                &VERSION,
            ));
        }

        Ok(Self)
    }
}
