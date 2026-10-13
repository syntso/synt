use std::io;
use std::path::PathBuf;

use nix::unistd::{self, User};

pub(crate) fn home_directory() -> io::Result<PathBuf> {
    let user = this_user()?;

    if user.dir.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the user database names no home folder for this user",
        ));
    }

    Ok(user.dir)
}

pub(crate) fn user_name() -> Option<String> {
    this_user().ok().map(|user| user.name)
}

fn this_user() -> io::Result<User> {
    User::from_uid(unistd::geteuid())
        .map_err(io::Error::from)?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "this user is missing from the user database",
            )
        })
}
