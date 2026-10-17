mod attributes;
mod descriptors;

use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io;
use std::iter;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::thread;

use nix::spawn::{PosixSpawnAttr, PosixSpawnFileActions, posix_spawn};
use nix::sys::wait::waitpid;

use attributes::DetachedAttributes;
use descriptors::DetachedDescriptors;

use super::{Environment, NativeSpawn};
use crate::{home_directory, user_name};

const NOWHERE: &str = "/dev/null";
const SYSTEM_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";
const PASSED_ON: [&str; 4] = ["LANG", "LC_ALL", "LC_CTYPE", "SSH_AUTH_SOCK"];

pub(super) struct Unix;

impl Unix {
    fn nowhere() -> io::Result<File> {
        OpenOptions::new().read(true).write(true).open(NOWHERE)
    }

    fn command_line(program: &Path, arguments: &[OsString]) -> io::Result<Vec<CString>> {
        iter::once(program.as_os_str())
            .chain(arguments.iter().map(OsString::as_os_str))
            .map(Self::c_string)
            .collect()
    }

    fn variables(environment: &Environment) -> io::Result<Vec<CString>> {
        environment
            .variables()
            .iter()
            .map(|variable| {
                let mut assignment = variable.name().to_owned();
                assignment.push("=");
                assignment.push(variable.value());

                Self::c_string(&assignment)
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
}

impl NativeSpawn for Unix {
    fn environment() -> io::Result<Environment> {
        let mut environment = Environment::default();

        environment.set("PATH", SYSTEM_PATH);
        environment.set("HOME", home_directory()?);

        if let Some(user) = user_name() {
            environment.set("USER", &user);
            environment.set("LOGNAME", user);
        }

        environment.pass_on(&PASSED_ON);

        Ok(environment)
    }

    fn spawn(program: &Path, arguments: &[OsString], environment: &Environment) -> io::Result<()> {
        let attributes = PosixSpawnAttr::detached()?;
        let nowhere = Self::nowhere()?;
        let descriptors = PosixSpawnFileActions::detached(&nowhere)?;
        let command_line = Self::command_line(program, arguments)?;
        let variables = Self::variables(environment)?;

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
}
