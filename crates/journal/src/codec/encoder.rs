use serde_json::{Number, Value};
use synt_proto::Actor;

use super::actor_tag::ActorTag;
use super::value_tag::ValueTag;
use crate::error::JournalError;

pub(crate) struct Encoder<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    pub(crate) fn actor_bytes(actor: &Actor) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        Encoder::new(&mut out).actor(actor);

        out
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.out.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn i64(&mut self, value: i64) {
        self.out.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn len(&mut self, len: usize) {
        let len = u32::try_from(len).unwrap_or(u32::MAX);
        self.out.extend_from_slice(&len.to_be_bytes());
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.len(value.len());
        self.out.extend_from_slice(value);
    }

    pub(crate) fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub(crate) fn optional<T>(&mut self, value: Option<T>, write: impl FnOnce(&mut Self, T)) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                write(self, value);
            }
        }
    }

    pub(crate) fn actor(&mut self, actor: &Actor) {
        self.tag(ActorTag::of(actor));

        match actor {
            Actor::Person | Actor::Core => {}
            Actor::Agent(name) | Actor::Runbook(name) | Actor::Plugin(name) => {
                self.str(name.as_str());
            }
        }
    }

    pub(crate) fn null(&mut self) {
        self.tag(ValueTag::Null);
    }

    pub(crate) fn value(&mut self, value: &Value) -> Result<(), JournalError> {
        self.value_at(value, 0)
    }

    fn value_at(&mut self, value: &Value, depth: usize) -> Result<(), JournalError> {
        if depth > ValueTag::MAX_DEPTH {
            return Err(JournalError::TooDeep {
                limit: ValueTag::MAX_DEPTH,
            });
        }

        match value {
            Value::Null => self.null(),
            Value::Bool(false) => self.tag(ValueTag::False),
            Value::Bool(true) => self.tag(ValueTag::True),
            Value::Number(number) => self.number(number)?,
            Value::String(text) => {
                self.tag(ValueTag::Text);
                self.str(text);
            }
            Value::Array(items) => self.list(items, depth)?,
            Value::Object(entries) => self.map(entries, depth)?,
        }

        Ok(())
    }

    fn number(&mut self, number: &Number) -> Result<(), JournalError> {
        if let Some(value) = number.as_i64() {
            self.tag(ValueTag::Signed);
            self.i64(value);
        } else if let Some(value) = number.as_u64() {
            self.tag(ValueTag::Unsigned);
            self.u64(value);
        } else {
            return Err(JournalError::Float);
        }

        Ok(())
    }

    fn list(&mut self, items: &[Value], depth: usize) -> Result<(), JournalError> {
        self.tag(ValueTag::List);
        self.len(items.len());

        for item in items {
            self.value_at(item, depth + 1)?;
        }

        Ok(())
    }

    fn map(
        &mut self,
        entries: &serde_json::Map<String, Value>,
        depth: usize,
    ) -> Result<(), JournalError> {
        let mut sorted: Vec<_> = entries.iter().collect();
        sorted.sort_unstable_by_key(|(key, _)| *key);

        self.tag(ValueTag::Map);
        self.len(sorted.len());

        for (key, value) in sorted {
            self.str(key);
            self.value_at(value, depth + 1)?;
        }

        Ok(())
    }

    fn tag(&mut self, tag: impl Into<u8>) {
        self.u8(tag.into());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use synt_proto::{Actor, Name};

    use super::Encoder;
    use crate::error::JournalError;

    fn value_bytes(value: &Value) -> Vec<u8> {
        let mut out = Vec::new();
        Encoder::new(&mut out).value(value).unwrap();
        out
    }

    #[test]
    fn prefixes_bytes_with_their_length() {
        let mut out = Vec::new();
        Encoder::new(&mut out).str("ab");

        assert_eq!(out, [0, 0, 0, 2, b'a', b'b']);
    }

    #[test]
    fn marks_absent_and_present_options() {
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out);

        encoder.optional(None::<u8>, Encoder::u8);
        encoder.optional(Some(7), Encoder::u8);

        assert_eq!(out, [0, 1, 7]);
    }

    #[test]
    fn key_order_does_not_change_the_bytes() {
        let one = serde_json::from_str::<Value>(r#"{"a":1,"b":2}"#).unwrap();
        let other = serde_json::from_str::<Value>(r#"{"b":2,"a":1}"#).unwrap();

        assert_eq!(value_bytes(&one), value_bytes(&other));
    }

    #[test]
    fn floats_are_refused() {
        let mut out = Vec::new();
        let refused = Encoder::new(&mut out).value(&json!({ "ratio": 0.5 }));

        assert!(matches!(refused, Err(JournalError::Float)));
    }

    #[test]
    fn deep_nesting_is_refused() {
        let mut value = json!(0);
        for _ in 0..=32 {
            value = json!([value]);
        }

        let mut out = Vec::new();
        assert!(Encoder::new(&mut out).value(&value).is_err());
    }

    #[test]
    fn different_actors_never_share_bytes() {
        let name = Name::new("x").unwrap();

        assert_ne!(
            Encoder::actor_bytes(&Actor::Agent(name.clone())),
            Encoder::actor_bytes(&Actor::Plugin(name))
        );
    }
}
