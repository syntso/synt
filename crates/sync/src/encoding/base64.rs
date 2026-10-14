use std::fmt;

use data_encoding::BASE64;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, PartialEq, Eq, Default)]
pub struct Base64(Vec<u8>);

impl Base64 {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn get(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub(crate) fn decode(text: &str) -> Result<Self, String> {
        BASE64
            .decode(text.as_bytes())
            .map(Self)
            .map_err(|error| format!("invalid base64: {error}"))
    }
}

impl fmt::Debug for Base64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Base64({} bytes)", self.0.len())
    }
}

impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&BASE64.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;

        Self::decode(&text).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Base64;

    #[test]
    fn bytes_travel_as_base64_text() {
        let bytes = Base64::new(b"synt".to_vec());

        let text = serde_json::to_string(&bytes).unwrap();

        assert_eq!(text, "\"c3ludA==\"");
        assert_eq!(serde_json::from_str::<Base64>(&text).unwrap(), bytes);
    }

    #[test]
    fn text_that_is_not_base64_is_refused() {
        assert!(serde_json::from_str::<Base64>("\"not base64!\"").is_err());
    }
}
