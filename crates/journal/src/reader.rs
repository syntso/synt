use std::io::Write;
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::Arc;

use crate::anchor::KeyStore;
use crate::chain::{JournalId, JournalKey, Linker};
use crate::error::JournalError;
use crate::export;
use crate::filter::Filter;
use crate::model::{Event, Seq};
use crate::store::ReadDb;
use crate::verify::{Verification, Walk};

pub struct JournalReader {
    id: JournalId,
    db: ReadDb,
    linker: Linker,
    keys: Arc<dyn KeyStore>,
}

impl JournalReader {
    pub fn open(
        path: impl AsRef<Path>,
        id: JournalId,
        keys: Arc<dyn KeyStore>,
    ) -> Result<Self, JournalError> {
        let path = path.as_ref();

        if !path.try_exists()? {
            return Err(match keys.anchors(id)? {
                Some(_) => JournalError::Missing(id),
                None => JournalError::NotFound(path.to_owned()),
            });
        }

        let db = ReadDb::open(path)?;
        let found = db.journal_id()?;

        if found != id {
            return Err(JournalError::Foreign {
                expected: id,
                found,
            });
        }

        let device = keys.device_key()?;
        let linker = Linker::new(&JournalKey::derive(&device, id));

        Ok(Self {
            id,
            db,
            linker,
            keys,
        })
    }

    pub fn id(&self) -> JournalId {
        self.id
    }

    pub fn get(&self, seq: Seq) -> Result<Option<Event>, JournalError> {
        self.db.get(seq)
    }

    pub fn page(
        &self,
        filter: &Filter,
        after: Option<Seq>,
        limit: u32,
    ) -> Result<Vec<Event>, JournalError> {
        self.db.page(filter, after.unwrap_or(Seq::ORIGIN), limit)
    }

    pub fn verify(&self) -> Result<Verification, JournalError> {
        let anchors = self
            .keys
            .anchors(self.id)?
            .ok_or(JournalError::AnchorsMissing(self.id))?;
        let mut walk = Walk::new(&self.linker, anchors.floor, anchors.head);

        let broken = self
            .db
            .scan(anchors.floor.seq, |row| match walk.step(row) {
                Ok(()) => ControlFlow::Continue(()),
                Err(broken) => ControlFlow::Break(broken),
            })?;

        Ok(broken.unwrap_or_else(|| walk.finish()))
    }

    pub fn export_jsonl(&self, filter: &Filter, out: impl Write) -> Result<u64, JournalError> {
        export::jsonl(self, filter, out)
    }
}
