use std::ffi::OsString;
use std::io;
use std::path::Path;

use super::Environment;

pub(super) trait NativeSpawn {
    fn environment() -> io::Result<Environment>;

    fn spawn(program: &Path, arguments: &[OsString], environment: &Environment) -> io::Result<()>;
}
