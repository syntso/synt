mod environment;
mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::ffi::OsString;
use std::io;
use std::path::Path;

use environment::Environment;
use native::NativeSpawn;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

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

    let environment = Os::environment()?;

    Os::spawn(program, arguments, &environment)
}
