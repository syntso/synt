use std::io;

pub trait Found<T> {
    fn found(self) -> io::Result<Option<T>>;
}

impl<T> Found<T> for io::Result<T> {
    fn found(self) -> io::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;

    use super::Found;

    #[test]
    fn a_missing_file_is_found_to_be_absent() {
        let folder = tempfile::tempdir().unwrap();

        let read = fs::read(folder.path().join("none")).found().unwrap();

        assert_eq!(read, None);
    }

    #[test]
    fn other_errors_stay_errors() {
        let failed: io::Result<()> = Err(io::Error::from(io::ErrorKind::PermissionDenied));

        assert!(failed.found().is_err());
    }
}
