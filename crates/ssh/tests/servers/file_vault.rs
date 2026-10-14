use std::fs;
use std::path::PathBuf;

use synt_ssh::KeyVault;
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
    fn private_key(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
        if name != self.name {
            return Ok(None);
        }

        fs::read(&self.path)
            .map(|bytes| Some(Zeroizing::new(bytes)))
            .map_err(|error| error.to_string())
    }
}
