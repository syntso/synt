use std::io;
use std::path::PathBuf;

pub(crate) fn ssh_directory() -> io::Result<PathBuf> {
    Ok(synt_platform::home_directory()?.join(".ssh"))
}

pub fn local_user() -> Option<String> {
    synt_platform::user_name()
}
