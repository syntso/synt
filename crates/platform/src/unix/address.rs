use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{self, Path, PathBuf};

use crate::directories;

pub(crate) fn address(directory: &Path, name: &str) -> OsString {
    directory.join(format!("{name}.sock")).into_os_string()
}

pub(crate) fn default_address(name: &str) -> io::Result<OsString> {
    let project = directories::project()?;
    let directory = project
        .runtime_dir()
        .unwrap_or_else(|| project.data_local_dir());

    Ok(address(directory, name))
}

pub(crate) fn absolute_address(address: &OsStr) -> OsString {
    path::absolute(Path::new(address)).map_or_else(|_| address.to_owned(), PathBuf::into_os_string)
}
