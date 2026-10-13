use std::env;
use std::io;
use std::path::PathBuf;

pub(crate) fn ssh_directory() -> io::Result<PathBuf> {
    let home = env::home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no home directory for this user")
    })?;

    Ok(home.join(".ssh"))
}

pub(crate) fn local_user() -> Option<String> {
    ["USER", "USERNAME"]
        .into_iter()
        .find_map(|name| env::var(name).ok())
        .filter(|user| !user.is_empty())
}
