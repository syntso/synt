use crate::bindings::synt::extension::types::{Error, Level};

pub(crate) trait Imports: Send {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> Result<String, Error>;

    fn storage_get(&mut self, key: String) -> Option<Vec<u8>>;

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> Result<(), Error>;

    fn log(&mut self, level: Level, message: String);
}
