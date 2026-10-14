use std::error::Error;

use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct KeyStoreError(Box<dyn Error + Send + Sync>);

impl KeyStoreError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
