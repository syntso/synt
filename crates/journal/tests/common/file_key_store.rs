use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use synt_journal::{Anchors, DeviceKey, JournalId, KeyStore, KeyStoreError};

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
        let bytes = fs::read(self.key_path()).map_err(KeyStoreError::new)?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| KeyStoreError::new("bad key file"))?;

        Ok(DeviceKey::from_bytes(bytes))
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        if self.key_path().exists() {
            return self.device_key();
        }

        let key = DeviceKey::generate().map_err(KeyStoreError::new)?;
        Self::write_atomically(&self.key_path(), key.expose()).map_err(KeyStoreError::new)?;

        Ok(key)
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError> {
        let Ok(bytes) = fs::read(self.anchors_path(journal)) else {
            return Ok(None);
        };

        Anchors::from_bytes(&bytes)
            .map(Some)
            .ok_or_else(|| KeyStoreError::new("bad anchors file"))
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError> {
        Self::write_atomically(&self.anchors_path(journal), &anchors.to_bytes())
            .map_err(KeyStoreError::new)
    }
}
