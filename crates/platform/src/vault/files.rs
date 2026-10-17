use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use super::{KeyPlace, Vault};
use crate::{Found, PlatformError, create_private_directory};

const FILE_MODE: u32 = 0o600;
const PENDING_SUFFIX: &str = ".pending";

pub(super) struct Files {
    directory: PathBuf,
}

impl Files {
    pub(super) fn open(directory: PathBuf) -> Result<Self, PlatformError> {
        create_private_directory(&directory)?;

        Ok(Self { directory })
    }

    fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
        let pending = Self::pending(path);
        fs::remove_file(&pending).found()?;

        Self::write_new(&pending, bytes)?;
        fs::rename(&pending, path)?;

        Self::sync_directory(path)
    }

    fn pending(path: &Path) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(PENDING_SUFFIX);

        PathBuf::from(name)
    }

    fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .open(path)?;

        file.set_permissions(Permissions::from_mode(FILE_MODE))?;
        file.write_all(bytes)?;

        file.sync_all()
    }

    fn sync_directory(path: &Path) -> io::Result<()> {
        let Some(directory) = path.parent() else {
            return Ok(());
        };

        File::open(directory)?.sync_all()
    }
}

impl Vault for Files {
    fn place(&self) -> KeyPlace {
        KeyPlace::Files(self.directory.clone())
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        let bytes = fs::read(self.directory.join(name)).found()?;

        Ok(bytes.map(Zeroizing::new))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        Ok(Self::replace(&self.directory.join(name), bytes)?)
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        fs::remove_file(self.directory.join(name)).found()?;

        Ok(())
    }
}
