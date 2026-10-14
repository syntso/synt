use std::io;

use nix::unistd;

use super::{NativeUser, UserId};

pub(super) struct Unix;

impl NativeUser for Unix {
    fn current() -> io::Result<UserId> {
        Ok(UserId::new(unistd::geteuid().as_raw()))
    }
}
