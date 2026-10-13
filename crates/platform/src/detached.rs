use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use crate::native;

pub fn start_detached(program: &Path, arguments: &[OsString]) -> io::Result<()> {
    let folder = program_folder(program)?;
    let environment = native::detached_environment()?;

    let mut command = Command::new(program);
    command
        .args(arguments)
        .env_clear()
        .envs(environment)
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    native::detach(&mut command);

    let mut child = command.spawn()?;
    thread::spawn(move || child.wait());

    Ok(())
}

fn program_folder(program: &Path) -> io::Result<&Path> {
    if !program.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is not an absolute path, so it is not started",
                program.display()
            ),
        ));
    }

    program.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no folder", program.display()),
        )
    })
}
