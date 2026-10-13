use std::env;
use std::ffi::OsString;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use super::account::home_directory;

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const SYSTEM_ROOT: &str = "SystemRoot";
const DEFAULT_SYSTEM_ROOT: &str = r"C:\Windows";
const PASSED_ON: [&str; 6] = [
    "SystemDrive",
    "TEMP",
    "TMP",
    "USERNAME",
    "USERDOMAIN",
    "windir",
];

pub(crate) fn detach(command: &mut Command) {
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

pub(crate) fn detached_environment() -> io::Result<Vec<(OsString, OsString)>> {
    let system_root = system_root();
    let path = env::join_paths([system_root.join("System32"), system_root.clone()])
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

    let mut environment = vec![
        (OsString::from(SYSTEM_ROOT), system_root.into_os_string()),
        (OsString::from("PATH"), path),
        (
            OsString::from("USERPROFILE"),
            home_directory()?.into_os_string(),
        ),
    ];

    environment.extend(passed_on());

    Ok(environment)
}

pub(crate) fn system_root() -> PathBuf {
    env::var_os(SYSTEM_ROOT)
        .map(PathBuf::from)
        .filter(|root| root.is_absolute())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SYSTEM_ROOT))
}

fn passed_on() -> impl Iterator<Item = (OsString, OsString)> {
    PASSED_ON
        .into_iter()
        .filter_map(|name| Some((OsString::from(name), env::var_os(name)?)))
}
