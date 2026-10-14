use std::ffi::OsString;
use std::io;
use std::path::Path;

use crate::native;

pub fn start_detached(program: &Path, arguments: &[OsString]) -> io::Result<()> {
    if !program.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is not an absolute path, so it is not started",
                program.display()
            ),
        ));
    }

    let environment = native::detached_environment()?;

    native::spawn_detached(program, arguments, &environment)
}
