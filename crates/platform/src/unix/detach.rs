use std::env;
use std::ffi::OsString;
use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use super::account::{home_directory, user_name};

const SYSTEM_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";
const PASSED_ON: [&str; 4] = ["LANG", "LC_ALL", "LC_CTYPE", "SSH_AUTH_SOCK"];

pub(crate) fn detach(command: &mut Command) {
    command.process_group(0);
}

pub(crate) fn detached_environment() -> io::Result<Vec<(OsString, OsString)>> {
    let mut environment = vec![
        (OsString::from("PATH"), OsString::from(SYSTEM_PATH)),
        (OsString::from("HOME"), home_directory()?.into_os_string()),
    ];

    if let Some(user) = user_name() {
        environment.push((OsString::from("USER"), OsString::from(&user)));
        environment.push((OsString::from("LOGNAME"), OsString::from(user)));
    }

    environment.extend(passed_on());

    Ok(environment)
}

fn passed_on() -> impl Iterator<Item = (OsString, OsString)> {
    PASSED_ON
        .into_iter()
        .filter_map(|name| Some((OsString::from(name), env::var_os(name)?)))
}
