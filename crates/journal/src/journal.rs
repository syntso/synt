use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::anchor::{Anchors, KeyStore, Mark};
use crate::builder::JournalBuilder;
use crate::chain::{Fields, JournalId, Linker};
use crate::error::JournalError;
use crate::lock::WriterLock;
use crate::model::{Clock, NewEvent, Seq, Timestamp};
use crate::pruned::Pruned;
use crate::reader::JournalReader;
use crate::store::WriteDb;
use crate::verify::{Damage, Verification, Walk};

const SCRATCH_BYTES: usize = 512;

pub struct Journal {
    id: JournalId,
    path: PathBuf,
    db: WriteDb,
    linker: Linker,
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

    pub(crate) fn create_with(
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
        let linker = Linker::derive(&device, id)?;
        let anchors = Anchors::genesis(linker.genesis());

        let db = WriteDb::create(path, id)?;
        keys.save_anchors(id, &anchors)?;

        Ok(Self {
            id,
            path: path.to_owned(),
            db,
            linker,
            keys,
            clock,
            anchors,
            last_at: Timestamp::EPOCH,
            scratch: Vec::with_capacity(SCRATCH_BYTES),
            _lock: lock,
        })
    }

    pub(crate) fn open_with(
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

        let db = WriteDb::open(path, id)?;
        let linker = Linker::derive(&keys.device_key()?, id)?;
        let tail = db.tail()?;

        let anchors = known_anchors(stored, tail.is_none(), &linker, id)?;
        let head = recover(&db, &linker, anchors)?;
        let anchors = anchors.with_head(head);

        if stored != Some(anchors) {
            keys.save_anchors(id, &anchors)?;
        }

        Ok(Self {
            id,
            path: path.to_owned(),
            db,
            linker,
            keys,
            clock,
            anchors,
            last_at: tail.map_or(Timestamp::EPOCH, |(_, at)| at),
            scratch: Vec::with_capacity(SCRATCH_BYTES),
            _lock: lock,
        })
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

        let Some(floor) = self.db.last_before(older_than, self.anchors.head.seq)? else {
            return Ok(unchanged);
        };
        if floor.seq <= self.anchors.floor.seq {
            return Ok(unchanged);
        }

        self.check_through(floor)?;

        let anchors = self.anchors.with_floor(floor);
        self.keys.save_anchors(self.id, &anchors)?;
        self.anchors = anchors;

        let removed = self.db.delete_through(floor.seq)?;
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

    fn commit(&mut self, events: &[NewEvent]) -> Result<Seq, JournalError> {
        let mut head = self.anchors.head;
        let mut at = self.last_at;
        let mut rows = Vec::with_capacity(events.len());

        for event in events {
            at = self.clock.now().max(at);
            let seq = head.seq.next();

            let fields = Fields::of(event, seq, at);
            fields.write(&mut self.scratch);
            let link = self.linker.link(&head.link, &self.scratch);

            rows.push((fields, link));
            head = Mark::new(seq, link);
        }

        self.db.insert(rows)?;

        self.anchors = self.anchors.with_head(head);
        self.last_at = at;
        self.keys.save_anchors(self.id, &self.anchors)?;

        Ok(head.seq)
    }

    fn check_through(&self, floor: Mark) -> Result<(), JournalError> {
        let mut walk = Walk::new(&self.linker, self.anchors.floor, floor);

        let broken = self.db.scan(self.anchors.floor.seq, |row| {
            if row.seq > floor.seq {
                return ControlFlow::Break(None);
            }

            match walk.step(row) {
                Ok(()) => ControlFlow::Continue(()),
                Err(broken) => ControlFlow::Break(Some(broken)),
            }
        })?;

        match broken.flatten() {
            Some(broken) => Err(JournalError::Tampered(broken)),
            None if walk.last() != floor => Err(JournalError::Tampered(Verification::Broken {
                at: walk.last().seq.next(),
                damage: Damage::Missing,
            })),
            None => Ok(()),
        }
    }
}

fn known_anchors(
    stored: Option<Anchors>,
    empty: bool,
    linker: &Linker,
    id: JournalId,
) -> Result<Anchors, JournalError> {
    match stored {
        Some(anchors) => Ok(anchors),
        None if empty => Ok(Anchors::genesis(linker.genesis())),
        None => Err(JournalError::AnchorsMissing(id)),
    }
}

fn recover(db: &WriteDb, linker: &Linker, anchors: Anchors) -> Result<Mark, JournalError> {
    let start = before_head(db, anchors)?;
    let mut walk = Walk::new(linker, start, anchors.head);

    let broken = db.scan(start.seq, |row| match walk.step(row) {
        Ok(()) => ControlFlow::Continue(()),
        Err(broken) => ControlFlow::Break(broken),
    })?;

    if let Some(broken) = broken {
        return Err(JournalError::Tampered(broken));
    }

    if walk.last().seq < anchors.head.seq {
        return Err(JournalError::Tampered(walk.finish()));
    }

    Ok(walk.last())
}

fn before_head(db: &WriteDb, anchors: Anchors) -> Result<Mark, JournalError> {
    let head = anchors.head;

    if head.seq <= anchors.floor.seq {
        return Ok(head);
    }

    let previous = Seq::new(head.seq.get() - 1);

    if previous == anchors.floor.seq {
        return Ok(anchors.floor);
    }

    match db.link_at(previous)? {
        Some(link) => Ok(Mark::new(previous, link)),
        None => Err(JournalError::Tampered(missing_before_head(db, previous)?)),
    }
}

fn missing_before_head(db: &WriteDb, previous: Seq) -> Result<Verification, JournalError> {
    let last = db.tail()?.map_or(Seq::ORIGIN, |(seq, _)| seq);

    if last < previous {
        return Ok(Verification::Truncated {
            last,
            head: previous.next(),
        });
    }

    Ok(Verification::Broken {
        at: previous,
        damage: Damage::Missing,
    })
}
