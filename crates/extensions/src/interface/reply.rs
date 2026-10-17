use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply(String);

impl Reply {
    pub(crate) fn new(text: String) -> Self {
        Self(text)
    }

    pub(crate) fn into_value(self) -> Value {
        serde_json::from_str(&self.0).unwrap_or(Value::String(self.0))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::Reply;

    #[test]
    fn json_stays_json_and_other_text_becomes_a_string() {
        assert_eq!(
            Reply::new("{\"a\":1}".to_owned()).into_value(),
            json!({ "a": 1 })
        );
        assert_eq!(
            Reply::new("plain".to_owned()).into_value(),
            Value::String("plain".to_owned())
        );
    }
}
