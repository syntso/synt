use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::native;

pub(crate) fn project() -> io::Result<ProjectDirs> {
    ProjectDirs::from("so", "synt", "Synt")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory for this user"))
}

pub fn data_directory() -> io::Result<PathBuf> {
    Ok(project()?.data_local_dir().to_owned())
}

pub fn create_private_directory(path: &Path) -> io::Result<()> {
    native::create_private_directory(path)
}
