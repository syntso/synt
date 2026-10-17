use std::fs;
use std::path::PathBuf;

use synt_ssh::{KeyVault, SshError};
use zeroize::Zeroizing;

pub struct FileVault {
    name: String,
    path: PathBuf,
}

impl FileVault {
    pub fn holding(name: &str, path: PathBuf) -> Self {
        Self {
            name: name.to_owned(),
            path,
        }
    }
}

impl KeyVault for FileVault {
    fn private_key(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SshError> {
        if name != self.name {
            return Ok(None);
        }

        let bytes = fs::read(&self.path).map_err(SshError::key_vault)?;

        Ok(Some(Zeroizing::new(bytes)))
    }
}
