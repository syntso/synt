use synt_proto::ResourceRef;

use crate::codec::Encoder;
use crate::event::NewEvent;
use crate::seq::Seq;
use crate::timestamp::Timestamp;

const VERSION: u8 = 1;

pub(crate) struct Fields<'a> {
    pub(crate) seq: Seq,
    pub(crate) at: Timestamp,
    pub(crate) actor: &'a [u8],
    pub(crate) kind: u8,
    pub(crate) call: Option<Seq>,
    pub(crate) action: &'a str,
    pub(crate) resource: Option<&'a str>,
    pub(crate) details: &'a [u8],
}

impl<'a> Fields<'a> {
    pub(crate) fn of(event: &'a NewEvent, seq: Seq, at: Timestamp) -> Self {
        Self {
            seq,
            at,
            actor: &event.actor_bytes,
            kind: event.kind.code(),
            call: event.call,
            action: event.action.as_str(),
            resource: event.resource.as_ref().map(ResourceRef::as_str),
            details: event.details.bytes(),
        }
    }

    pub(crate) fn write(&self, out: &mut Vec<u8>) {
        out.clear();
        let mut encoder = Encoder::new(out);

        encoder.u8(VERSION);
        encoder.u64(self.seq.get());
        encoder.i64(self.at.as_millis());
        encoder.bytes(self.actor);
        encoder.u8(self.kind);
        encoder.optional(self.call, |encoder, call| encoder.u64(call.get()));
        encoder.str(self.action);
        encoder.optional(self.resource, Encoder::str);
        encoder.bytes(self.details);
    }
}
