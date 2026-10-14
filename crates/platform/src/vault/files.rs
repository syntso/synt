use std::fs;
use std::path::PathBuf;

use zeroize::Zeroizing;

use super::KeyPlace;
use super::Vault;
use crate::files::{found, remove_if_present};
use crate::{PlatformError, native};

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
        let bytes = found(fs::read(self.directory.join(name)))?;

        Ok(bytes.map(Zeroizing::new))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        Ok(native::replace_private_file(
            &self.directory.join(name),
            bytes,
        )?)
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        Ok(remove_if_present(&self.directory.join(name))?)
    }
}
