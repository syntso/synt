use serde_json::{Map, Value};
use synt_proto::{Actor, Name};

use super::actor_tag::ActorTag;
use super::value_tag::ValueTag;
use crate::error::JournalError;

pub(crate) struct Decoder<'a> {
    input: &'a [u8],
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    pub(crate) fn u8(&mut self) -> Result<u8, JournalError> {
        let [value] = self.take_array::<1>()?;
        Ok(value)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, JournalError> {
        Ok(u64::from_be_bytes(self.take_array()?))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, JournalError> {
        Ok(i64::from_be_bytes(self.take_array()?))
    }

    pub(crate) fn len(&mut self) -> Result<usize, JournalError> {
        let len = u32::from_be_bytes(self.take_array()?);
        usize::try_from(len).map_err(|_| JournalError::Malformed("length does not fit in memory"))
    }

    pub(crate) fn bytes(&mut self) -> Result<&'a [u8], JournalError> {
        let len = self.len()?;
        self.take(len)
    }

    pub(crate) fn str(&mut self) -> Result<&'a str, JournalError> {
        let bytes = self.bytes()?;
        std::str::from_utf8(bytes).map_err(|_| JournalError::Malformed("text is not utf-8"))
    }

    pub(crate) fn actor(mut self) -> Result<Actor, JournalError> {
        let tag = ActorTag::try_from(self.u8()?)?;

        let actor = match tag {
            ActorTag::Person => Actor::Person,
            ActorTag::Core => Actor::Core,
            ActorTag::Agent => Actor::Agent(self.name()?),
            ActorTag::Runbook => Actor::Runbook(self.name()?),
            ActorTag::Plugin => Actor::Plugin(self.name()?),
        };

        self.finish()?;
        Ok(actor)
    }

    pub(crate) fn value(mut self) -> Result<Value, JournalError> {
        let value = self.value_at(0)?;

        self.finish()?;
        Ok(value)
    }

    pub(crate) fn finish(self) -> Result<(), JournalError> {
        if !self.input.is_empty() {
            return Err(JournalError::Malformed("trailing bytes"));
        }

        Ok(())
    }

    fn name(&mut self) -> Result<Name, JournalError> {
        let text = self.str()?;
        Name::new(text).map_err(|_| JournalError::Malformed("invalid actor name"))
    }

    fn value_at(&mut self, depth: usize) -> Result<Value, JournalError> {
        if depth > ValueTag::MAX_DEPTH {
            return Err(JournalError::Malformed("details nest too deep"));
        }

        let value = match ValueTag::try_from(self.u8()?)? {
            ValueTag::Null => Value::Null,
            ValueTag::False => Value::Bool(false),
            ValueTag::True => Value::Bool(true),
            ValueTag::Signed => Value::from(self.i64()?),
            ValueTag::Unsigned => self.unsigned()?,
            ValueTag::Text => Value::String(self.str()?.to_owned()),
            ValueTag::List => self.list(depth)?,
            ValueTag::Map => self.map(depth)?,
        };

        Ok(value)
    }

    fn unsigned(&mut self) -> Result<Value, JournalError> {
        let value = self.u64()?;

        if i64::try_from(value).is_ok() {
            return Err(JournalError::Malformed(
                "small unsigned number must be stored as signed",
            ));
        }

        Ok(Value::from(value))
    }

    fn list(&mut self, depth: usize) -> Result<Value, JournalError> {
        let len = self.len()?;
        let mut items = Vec::new();

        for _ in 0..len {
            items.push(self.value_at(depth + 1)?);
        }

        Ok(Value::Array(items))
    }

    fn map(&mut self, depth: usize) -> Result<Value, JournalError> {
        let len = self.len()?;
        let mut entries = Map::new();
        let mut previous: Option<&str> = None;

        for _ in 0..len {
            let key = self.str()?;

            if previous.is_some_and(|previous| previous >= key) {
                return Err(JournalError::Malformed("map keys out of order"));
            }

            let value = self.value_at(depth + 1)?;
            entries.insert(key.to_owned(), value);
            previous = Some(key);
        }

        Ok(Value::Object(entries))
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], JournalError> {
        if len > self.input.len() {
            return Err(JournalError::Malformed("unexpected end of input"));
        }

        let (taken, rest) = self.input.split_at(len);
        self.input = rest;

        Ok(taken)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], JournalError> {
        let bytes = self.take(N)?;
        bytes
            .try_into()
            .map_err(|_| JournalError::Malformed("unexpected end of input"))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_proto::{Actor, Name};

    use super::Decoder;
    use crate::codec::Encoder;

    fn name(text: &str) -> Name {
        Name::new(text).unwrap()
    }

    #[test]
    fn reads_back_what_was_written() {
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out);
        encoder.u64(42);
        encoder.i64(-7);
        encoder.str("synt");

        let mut decoder = Decoder::new(&out);

        assert_eq!(decoder.u64().unwrap(), 42);
        assert_eq!(decoder.i64().unwrap(), -7);
        assert_eq!(decoder.str().unwrap(), "synt");
        assert!(decoder.finish().is_ok());
    }

    #[test]
    fn refuses_lengths_beyond_the_input() {
        let mut decoder = Decoder::new(&[0, 0, 0, 9, 1]);
        assert!(decoder.bytes().is_err());
    }

    #[test]
    fn refuses_trailing_bytes() {
        let mut decoder = Decoder::new(&[1, 2]);
        decoder.u8().ok();

        assert!(decoder.finish().is_err());
    }

    #[test]
    fn every_actor_round_trips() {
        let actors = [
            Actor::Person,
            Actor::Core,
            Actor::Agent(name("agent-1")),
            Actor::Runbook(name("deploy-api")),
            Actor::Plugin(name("k8s")),
        ];

        for actor in actors {
            let mut out = Vec::new();
            Encoder::new(&mut out).actor(&actor);

            assert_eq!(Decoder::new(&out).actor().unwrap(), actor);
        }
    }

    #[test]
    fn unknown_actor_tags_are_refused() {
        assert!(Decoder::new(&[99]).actor().is_err());
    }

    #[test]
    fn values_round_trip() {
        let value = json!({
            "command": "systemctl restart api",
            "exit": 0,
            "bytes": u64::MAX,
            "lines": [-1, 2, null, true, false],
            "nested": { "b": 1, "a": "x" }
        });

        let mut out = Vec::new();
        Encoder::new(&mut out).value(&value).unwrap();

        assert_eq!(Decoder::new(&out).value().unwrap(), value);
    }

    #[test]
    fn deep_nesting_is_refused() {
        let crafted: Vec<u8> = std::iter::repeat_n([6, 0, 0, 0, 1], 34).flatten().collect();

        assert!(Decoder::new(&crafted).value().is_err());
    }

    #[test]
    fn non_canonical_bytes_are_refused() {
        let unsorted = [7, 0, 0, 0, 2, 0, 0, 0, 1, b'b', 0, 0, 0, 0, 1, b'a', 0];
        assert!(Decoder::new(&unsorted).value().is_err());

        let small_unsigned = [4, 0, 0, 0, 0, 0, 0, 0, 1];
        assert!(Decoder::new(&small_unsigned).value().is_err());
    }

    #[test]
    fn unknown_value_tags_are_refused() {
        assert!(Decoder::new(&[42]).value().is_err());
    }
}
