use std::io;
use std::path::Path;

use crate::SyncError;

pub(super) trait AtPath<T> {
    fn at(self, path: &Path) -> Result<T, SyncError>;
}

impl<T> AtPath<T> for io::Result<T> {
    fn at(self, path: &Path) -> Result<T, SyncError> {
        self.map_err(|error| SyncError::provider(format!("{}: {error}", path.display())))
    }
}
