use std::fs;
use std::io;
use std::path::Path;

pub(crate) fn found<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn remove_if_present(path: &Path) -> io::Result<()> {
    found(fs::remove_file(path))?;

    Ok(())
}
