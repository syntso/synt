use std::io;

use super::UserId;

pub(super) trait NativeUser {
    fn current() -> io::Result<UserId>;
}
