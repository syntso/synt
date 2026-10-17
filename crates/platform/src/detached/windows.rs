use std::env;
use std::ffi::OsString;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use super::{Environment, NativeSpawn};
use crate::directories::system_root;
use crate::home_directory;

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const PASSED_ON: [&str; 6] = [
    "SystemDrive",
    "TEMP",
    "TMP",
    "USERNAME",
    "USERDOMAIN",
    "windir",
];

pub(super) struct Windows;

impl NativeSpawn for Windows {
    fn environment() -> io::Result<Environment> {
        let system_root = system_root();
        let path = env::join_paths([system_root.join("System32"), system_root.clone()])
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

        let mut environment = Environment::default();

        environment.set("SystemRoot", system_root);
        environment.set("PATH", path);
        environment.set("USERPROFILE", home_directory()?);
        environment.pass_on(&PASSED_ON);

        Ok(environment)
    }

    fn spawn(program: &Path, arguments: &[OsString], environment: &Environment) -> io::Result<()> {
        let folder = program.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} has no folder", program.display()),
            )
        })?;

        let variables = environment
            .variables()
            .iter()
            .map(|variable| (variable.name(), variable.value()));

        let mut command = Command::new(program);
        command
            .args(arguments)
            .env_clear()
            .envs(variables)
            .current_dir(folder)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);

        let mut child = command.spawn()?;
        thread::spawn(move || child.wait());

        Ok(())
    }
}
