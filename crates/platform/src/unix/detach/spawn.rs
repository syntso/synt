use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io;
use std::iter;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::thread;

use nix::spawn::posix_spawn;
use nix::sys::wait::waitpid;

use super::attributes::detached_attributes;
use super::descriptors::detached_descriptors;

const NOWHERE: &str = "/dev/null";

pub(crate) fn spawn_detached(
    program: &Path,
    arguments: &[OsString],
    environment: &[(OsString, OsString)],
) -> io::Result<()> {
    let attributes = detached_attributes()?;
    let nowhere = nowhere()?;
    let descriptors = detached_descriptors(&nowhere)?;
    let command_line = command_line(program, arguments)?;
    let variables = variables(environment)?;

    let child = posix_spawn(
        program,
        &descriptors,
        &attributes,
        &command_line,
        &variables,
    )?;

    thread::spawn(move || waitpid(child, None));

    Ok(())
}

fn nowhere() -> io::Result<File> {
    OpenOptions::new().read(true).write(true).open(NOWHERE)
}

fn command_line(program: &Path, arguments: &[OsString]) -> io::Result<Vec<CString>> {
    iter::once(program.as_os_str())
        .chain(arguments.iter().map(OsString::as_os_str))
        .map(c_string)
        .collect()
}

fn variables(environment: &[(OsString, OsString)]) -> io::Result<Vec<CString>> {
    environment
        .iter()
        .map(|(name, value)| {
            let mut variable = name.clone();
            variable.push("=");
            variable.push(value);
            c_string(&variable)
        })
        .collect()
}

fn c_string(text: &OsStr) -> io::Result<CString> {
    CString::new(text.as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} contains a NUL byte", text.to_string_lossy()),
        )
    })
}
