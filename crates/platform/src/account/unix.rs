use std::io;
use std::path::PathBuf;

use nix::unistd::{self, User};

use super::NativeAccount;

pub(super) struct Unix;

impl Unix {
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
}

impl NativeAccount for Unix {
    fn home_directory() -> io::Result<PathBuf> {
        let user = Self::this_user()?;

        if user.dir.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the user database names no home folder for this user",
            ));
        }

        Ok(user.dir)
    }

    fn user_name() -> Option<String> {
        Self::this_user().ok().map(|user| user.name)
    }
}
