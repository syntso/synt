use std::env;
use std::io;
use std::path::PathBuf;

use directories::BaseDirs;

pub(crate) fn home_directory() -> io::Result<PathBuf> {
    let base = BaseDirs::new()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home folder for this user"))?;

    Ok(base.home_dir().to_owned())
}

pub(crate) fn user_name() -> Option<String> {
    env::var("USERNAME").ok().filter(|name| !name.is_empty())
}
