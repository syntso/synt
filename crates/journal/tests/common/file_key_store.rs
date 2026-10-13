use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_journal::{Anchors, DeviceKey, JournalId, KeyStore, KeyStoreError, Link, Mark, Seq};

pub struct FileKeyStore {
    dir: PathBuf,
}

impl FileKeyStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
        }
    }

    fn key_path(&self) -> PathBuf {
        self.dir.join("device.key")
    }

    fn anchors_path(&self, journal: JournalId) -> PathBuf {
        self.dir.join(format!("{journal}.anchors"))
    }

    fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
        let temporary = path.with_extension("tmp");

        fs::write(&temporary, bytes)?;
        fs::rename(temporary, path)
    }
}

impl KeyStore for FileKeyStore {
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        let path = self.key_path();

        if let Ok(bytes) = fs::read(&path) {
            let bytes: [u8; 32] = bytes
                .try_into()
                .map_err(|_| KeyStoreError::new("bad key file"))?;
            return Ok(DeviceKey::from_bytes(bytes));
        }

        let key = DeviceKey::generate().map_err(KeyStoreError::new)?;
        Self::write_atomically(&path, key.expose()).map_err(KeyStoreError::new)?;

        Ok(key)
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError> {
        let Ok(bytes) = fs::read(self.anchors_path(journal)) else {
            return Ok(None);
        };

        let mark = |offset: usize| -> Option<Mark> {
            let seq = u64::from_be_bytes(bytes.get(offset..offset + 8)?.try_into().ok()?);
            let link: [u8; 32] = bytes.get(offset + 8..offset + 40)?.try_into().ok()?;

            Some(Mark::new(Seq::new(seq), Link::from_bytes(link)))
        };

        let floor = mark(0).ok_or_else(|| KeyStoreError::new("bad anchors file"))?;
        let head = mark(40).ok_or_else(|| KeyStoreError::new("bad anchors file"))?;

        Ok(Some(Anchors { floor, head }))
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError> {
        let mut bytes = Vec::with_capacity(80);

        for mark in [anchors.floor, anchors.head] {
            bytes.extend_from_slice(&mark.seq.get().to_be_bytes());
            bytes.extend_from_slice(mark.link.as_bytes());
        }

        Self::write_atomically(&self.anchors_path(journal), &bytes).map_err(KeyStoreError::new)
    }
}
