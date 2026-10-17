use std::io::{self, BufWriter, Write};
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::Arc;

use super::JournalId;
use crate::chain::Chain;
use crate::error::JournalError;
use crate::event::Event;
use crate::filter::Filter;
use crate::key_store::KeyStore;
use crate::seq::Seq;
use crate::store::{ReadOnly, Store};
use crate::verify::{Verification, Walk};

const EXPORT_PAGE: u32 = 1_000;

pub struct JournalReader {
    id: JournalId,
    store: Store<ReadOnly>,
    chain: Chain,
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
            let anchored = keys.anchors(id)?.is_some();
            return Err(JournalError::no_file(path, id, anchored));
        }

        let store = Store::open(path, id)?;
        let chain = Chain::derive(&keys.device_key()?, id)?;

        Ok(Self {
            id,
            store,
            chain,
            keys,
        })
    }

    pub fn id(&self) -> JournalId {
        self.id
    }

    pub fn get(&self, seq: Seq) -> Result<Option<Event>, JournalError> {
        self.store.get(seq)
    }

    pub fn page(
        &self,
        filter: &Filter,
        after: Option<Seq>,
        limit: u32,
    ) -> Result<Vec<Event>, JournalError> {
        self.store.page(filter, after.unwrap_or(Seq::ORIGIN), limit)
    }

    pub fn verify(&self) -> Result<Verification, JournalError> {
        let anchors = self
            .keys
            .anchors(self.id)?
            .ok_or(JournalError::AnchorsMissing(self.id))?;
        let mut walk = Walk::new(&self.chain, anchors.floor, anchors.head);

        let broken = self
            .store
            .scan(anchors.floor.seq, |row| match walk.step(row) {
                Ok(()) => ControlFlow::Continue(()),
                Err(broken) => ControlFlow::Break(broken),
            })?;

        Ok(broken.unwrap_or_else(|| walk.finish()))
    }

    pub fn export_jsonl(&self, filter: &Filter, out: impl Write) -> Result<u64, JournalError> {
        let mut out = BufWriter::new(out);
        let mut after = Seq::ORIGIN;
        let mut written = 0;

        loop {
            let page = self.page(filter, Some(after), EXPORT_PAGE)?;

            let Some(last) = page.last() else {
                break;
            };
            after = last.seq();

            written += Self::write_lines(&page, &mut out)?;
        }

        out.flush()?;
        Ok(written)
    }

    fn write_lines(events: &[Event], out: &mut impl Write) -> Result<u64, JournalError> {
        for event in events {
            serde_json::to_writer(&mut *out, event).map_err(io::Error::from)?;
            out.write_all(b"\n")?;
        }

        Ok(u64::try_from(events.len()).unwrap_or(u64::MAX))
    }
}
