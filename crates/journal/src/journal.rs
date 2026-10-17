mod builder;
mod id;
mod lock;
mod pruned;
mod reader;
mod recovery;
mod writer;

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use self::lock::WriterLock;
use self::recovery::Recovery;
use crate::anchors::{Anchors, Mark};
use crate::chain::{Chain, Fields};
use crate::clock::Clock;
use crate::error::JournalError;
use crate::event::NewEvent;
use crate::key_store::KeyStore;
use crate::seq::Seq;
use crate::store::{Entry, Store, Writable};
use crate::timestamp::Timestamp;
use crate::verify::{Damage, Verification, Walk};

pub use builder::JournalBuilder;
pub use id::JournalId;
pub use pruned::Pruned;
pub use reader::JournalReader;
pub use writer::JournalWriter;

const SCRATCH_BYTES: usize = 512;

pub struct Journal {
    id: JournalId,
    path: PathBuf,
    store: Store<Writable>,
    chain: Chain,
    keys: Arc<dyn KeyStore>,
    clock: Arc<dyn Clock>,
    anchors: Anchors,
    last_at: Timestamp,
    scratch: Vec<u8>,
    _lock: WriterLock,
}

impl Journal {
    pub fn create(path: impl AsRef<Path>, keys: Arc<dyn KeyStore>) -> Result<Self, JournalError> {
        JournalBuilder::new(keys).create(path)
    }

    pub fn open(
        path: impl AsRef<Path>,
        id: JournalId,
        keys: Arc<dyn KeyStore>,
    ) -> Result<Self, JournalError> {
        JournalBuilder::new(keys).open(path, id)
    }

    pub fn id(&self) -> JournalId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn anchors(&self) -> Anchors {
        self.anchors
    }

    pub fn append(&mut self, event: &NewEvent) -> Result<Seq, JournalError> {
        self.commit(std::slice::from_ref(event))
    }

    pub fn append_all(&mut self, events: &[NewEvent]) -> Result<Vec<Seq>, JournalError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let first = self.anchors.head.seq.next();
        let last = self.commit(events)?;

        Ok((first.get()..=last.get()).map(Seq::new).collect())
    }

    pub fn prune(&mut self, older_than: Timestamp) -> Result<Pruned, JournalError> {
        let unchanged = Pruned {
            removed: 0,
            floor: self.anchors.floor.seq,
        };

        let Some(floor) = self.store.last_before(older_than, self.anchors.head.seq)? else {
            return Ok(unchanged);
        };
        if floor.seq <= self.anchors.floor.seq {
            return Ok(unchanged);
        }

        self.check_through(floor)?;

        let anchors = self.anchors.with_floor(floor);
        self.keys.save_anchors(self.id, &anchors)?;
        self.anchors = anchors;

        let removed = self.store.delete_through(floor.seq)?;

        Ok(Pruned {
            removed,
            floor: floor.seq,
        })
    }

    pub fn reader(&self) -> Result<JournalReader, JournalError> {
        JournalReader::open(&self.path, self.id, Arc::clone(&self.keys))
    }

    pub(crate) fn keys(&self) -> Arc<dyn KeyStore> {
        Arc::clone(&self.keys)
    }

    fn create_with(
        path: &Path,
        id: JournalId,
        keys: Arc<dyn KeyStore>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, JournalError> {
        let lock = WriterLock::acquire(path)?;

        if path.try_exists()? {
            return Err(JournalError::AlreadyExists(path.to_owned()));
        }

        let device = keys.ensure_device_key()?;
        let chain = Chain::derive(&device, id)?;
        let anchors = Anchors::genesis(chain.genesis());

        let store = Store::create(path, id)?;
        keys.save_anchors(id, &anchors)?;

        Ok(Self {
            id,
            path: path.to_owned(),
            store,
            chain,
            keys,
            clock,
            anchors,
            last_at: Timestamp::EPOCH,
            scratch: Vec::with_capacity(SCRATCH_BYTES),
            _lock: lock,
        })
    }

    fn open_with(
        path: &Path,
        id: JournalId,
        keys: Arc<dyn KeyStore>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, JournalError> {
        let lock = WriterLock::acquire(path)?;
        let stored = keys.anchors(id)?;

        if !path.try_exists()? {
            return Err(JournalError::no_file(path, id, stored.is_some()));
        }

        let store = Store::<Writable>::open(path, id)?;
        let chain = Chain::derive(&keys.device_key()?, id)?;
        let tail = store.tail()?;

        let anchors = Recovery::new(&store, &chain).anchors(id, stored, tail)?;

        if stored != Some(anchors) {
            keys.save_anchors(id, &anchors)?;
        }

        Ok(Self {
            id,
            path: path.to_owned(),
            store,
            chain,
            keys,
            clock,
            anchors,
            last_at: tail.map_or(Timestamp::EPOCH, |tail| tail.at),
            scratch: Vec::with_capacity(SCRATCH_BYTES),
            _lock: lock,
        })
    }

    fn commit(&mut self, events: &[NewEvent]) -> Result<Seq, JournalError> {
        let mut head = self.anchors.head;
        let mut at = self.last_at;
        let mut entries = Vec::with_capacity(events.len());

        for event in events {
            at = self.clock.now().max(at);
            let seq = head.seq.next();

            let fields = Fields::of(event, seq, at);
            fields.write(&mut self.scratch);
            let link = self.chain.link(&head.link, &self.scratch);

            entries.push(Entry { fields, link });
            head = Mark::new(seq, link);
        }

        self.store.insert(entries)?;

        self.anchors = self.anchors.with_head(head);
        self.last_at = at;
        self.keys.save_anchors(self.id, &self.anchors)?;

        Ok(head.seq)
    }

    fn check_through(&self, floor: Mark) -> Result<(), JournalError> {
        let mut walk = Walk::new(&self.chain, self.anchors.floor, floor);

        let broken = self.store.scan(self.anchors.floor.seq, |row| {
            if row.seq > floor.seq {
                return ControlFlow::Break(None);
            }

            match walk.step(row) {
                Ok(()) => ControlFlow::Continue(()),
                Err(broken) => ControlFlow::Break(Some(broken)),
            }
        })?;

        if let Some(broken) = broken.flatten() {
            return Err(JournalError::Tampered(broken));
        }
        if walk.last() != floor {
            return Err(JournalError::Tampered(Verification::Broken {
                at: walk.last().seq.next(),
                damage: Damage::Missing,
            }));
        }

        Ok(())
    }
}
