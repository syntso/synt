use std::fs;
use std::io;
use std::path::PathBuf;

use zeroize::Zeroizing;

use super::place::KeyPlace;
use super::vault::Vault;
use crate::PlatformError;
use crate::native;

pub(crate) struct Files {
    directory: PathBuf,
}

impl Files {
    pub(crate) fn open(directory: PathBuf) -> Result<Self, PlatformError> {
        native::create_private_directory(&directory)?;

        Ok(Self { directory })
    }
}

impl Vault for Files {
    fn place(&self) -> KeyPlace {
        KeyPlace::Files(self.directory.clone())
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        match fs::read(self.directory.join(name)) {
            Ok(bytes) => Ok(Some(Zeroizing::new(bytes))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        Ok(native::replace_private_file(
            &self.directory.join(name),
            bytes,
        )?)
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        match fs::remove_file(self.directory.join(name)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
