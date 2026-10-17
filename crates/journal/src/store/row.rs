mod cells;

use rusqlite::Row;
use synt_proto::{ActionId, ResourceRef};

use self::cells::Cells;
use crate::chain::{Fields, Link};
use crate::codec::Decoder;
use crate::error::JournalError;
use crate::event::{Details, Event, EventKind};
use crate::seq::Seq;
use crate::timestamp::Timestamp;

pub(super) const COLUMNS: &str = "seq, at, actor, kind, call, action, resource, details, link";

const SEQ: usize = 0;
const AT: usize = 1;
const ACTOR: usize = 2;
const KIND: usize = 3;
const CALL: usize = 4;
const ACTION: usize = 5;
const RESOURCE: usize = 6;
const DETAILS: usize = 7;
const LINK: usize = 8;

pub(crate) struct RowView<'a> {
    pub(crate) seq: Seq,
    pub(crate) link: Option<Link>,
    at: i64,
    actor: &'a [u8],
    kind: i64,
    call: Option<i64>,
    action: &'a str,
    resource: Option<&'a str>,
    details: &'a [u8],
}

impl<'a> RowView<'a> {
    pub(crate) fn read(row: &'a Row<'_>) -> Result<Self, JournalError> {
        let cells = Cells::new(row);

        Ok(Self {
            seq: Self::seq(row.get(SEQ)?)?,
            at: row.get(AT)?,
            actor: cells.blob(ACTOR)?,
            kind: row.get(KIND)?,
            call: row.get(CALL)?,
            action: cells.text(ACTION)?,
            resource: cells.optional_text(RESOURCE)?,
            details: cells.blob(DETAILS)?,
            link: Link::from_slice(cells.blob(LINK)?),
        })
    }

    pub(crate) fn seq(value: i64) -> Result<Seq, JournalError> {
        Seq::from_sql(value).ok_or(JournalError::Corrupt {
            seq: Seq::ORIGIN,
            reason: "negative sequence number",
        })
    }

    pub(crate) fn link(seq: Seq, bytes: &[u8]) -> Result<Link, JournalError> {
        Link::from_slice(bytes).ok_or(JournalError::Corrupt {
            seq,
            reason: "link has the wrong length",
        })
    }

    pub(crate) fn fields(&self) -> Option<Fields<'a>> {
        let call = match self.call {
            Some(call) => Some(Seq::from_sql(call)?),
            None => None,
        };

        Some(Fields {
            seq: self.seq,
            at: Timestamp::from_millis(self.at),
            actor: self.actor,
            kind: u8::try_from(self.kind).ok()?,
            call,
            action: self.action,
            resource: self.resource,
            details: self.details,
        })
    }

    pub(crate) fn to_event(&self) -> Result<Event, JournalError> {
        let corrupt = |reason| JournalError::Corrupt {
            seq: self.seq,
            reason,
        };

        let fields = self.fields().ok_or(corrupt("field out of range"))?;
        let kind = EventKind::from_code(fields.kind).ok_or(corrupt("unknown event kind"))?;
        let actor = Decoder::new(self.actor)
            .actor()
            .map_err(|error| error.at(self.seq))?;
        let action = ActionId::new(self.action).map_err(|_| corrupt("invalid action id"))?;
        let resource = self
            .resource
            .map(ResourceRef::new)
            .transpose()
            .map_err(|_| corrupt("invalid resource reference"))?;
        let details = Details::decode(self.details.to_vec()).map_err(|error| error.at(self.seq))?;
        let link = self.link.ok_or(corrupt("link has the wrong length"))?;

        Ok(Event {
            seq: self.seq,
            at: fields.at,
            actor,
            kind,
            call: fields.call,
            action,
            resource,
            details,
            link,
        })
    }
}
