use rusqlite::Row;
use rusqlite::types::{FromSqlError, Type};

use crate::chain::{Fields, Link};
use crate::error::JournalError;
use crate::model::{ActionId, Actor, Event, EventKind, ResourceRef, Seq, Timestamp};
use crate::payload::Details;

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
        Ok(Self {
            seq: stored_seq(row.get(SEQ)?)?,
            at: row.get(AT)?,
            actor: blob(row, ACTOR)?,
            kind: row.get(KIND)?,
            call: row.get(CALL)?,
            action: text(row, ACTION)?,
            resource: optional_text(row, RESOURCE)?,
            details: blob(row, DETAILS)?,
            link: Link::from_slice(blob(row, LINK)?),
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
        let actor = Actor::decode(self.actor).map_err(|error| corrupt(error.0))?;
        let action = ActionId::new(self.action).map_err(|_| corrupt("invalid action id"))?;
        let resource = self
            .resource
            .map(ResourceRef::new)
            .transpose()
            .map_err(|_| corrupt("invalid resource reference"))?;
        let details = Details::decode(self.details.to_vec()).map_err(|error| corrupt(error.0))?;
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

pub(super) fn stored_seq(value: i64) -> Result<Seq, JournalError> {
    Seq::from_sql(value).ok_or(JournalError::Corrupt {
        seq: Seq::ORIGIN,
        reason: "negative sequence number",
    })
}

pub(super) fn stored_link(seq: Seq, bytes: &[u8]) -> Result<Link, JournalError> {
    Link::from_slice(bytes).ok_or(JournalError::Corrupt {
        seq,
        reason: "link has the wrong length",
    })
}

fn blob<'a>(row: &'a Row<'_>, index: usize) -> rusqlite::Result<&'a [u8]> {
    let value = row.get_ref(index)?;
    value
        .as_blob()
        .map_err(|error| conversion(index, Type::Blob, error))
}

fn text<'a>(row: &'a Row<'_>, index: usize) -> rusqlite::Result<&'a str> {
    let value = row.get_ref(index)?;
    value
        .as_str()
        .map_err(|error| conversion(index, Type::Text, error))
}

fn optional_text<'a>(row: &'a Row<'_>, index: usize) -> rusqlite::Result<Option<&'a str>> {
    let value = row.get_ref(index)?;
    value
        .as_str_or_null()
        .map_err(|error| conversion(index, Type::Text, error))
}

fn conversion(index: usize, expected: Type, error: FromSqlError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, expected, Box::new(error))
}
