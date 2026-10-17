use synt_extension_api::Error;
use synt_extension_api::serde_json::{self, Value};

pub struct Params(Value);

impl Params {
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.trim().is_empty() {
            return Ok(Self(Value::Null));
        }

        serde_json::from_str(text)
            .map(Self)
            .map_err(|error| Error::new("invalid_params", error.to_string()))
    }

    pub fn text(&self, name: &str) -> Result<&str, Error> {
        self.optional(name)
            .ok_or_else(|| Error::new("invalid_params", format!("{name} is missing")))
    }

    pub fn optional(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(Value::as_str)
    }

    pub fn number(&self, name: &str) -> Result<u32, Error> {
        self.0
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok())
            .ok_or_else(|| Error::new("invalid_params", format!("{name} is not a number")))
    }

    pub fn value(&self, name: &str) -> Value {
        self.0.get(name).cloned().unwrap_or(Value::Null)
    }
}
