mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use native::NativeDirectories;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

pub(crate) fn project() -> io::Result<ProjectDirs> {
    ProjectDirs::from("so", "synt", "Synt")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory for this user"))
}

pub fn data_directory() -> io::Result<PathBuf> {
    Ok(project()?.data_local_dir().to_owned())
}

pub fn create_private_directory(path: &Path) -> io::Result<()> {
    Os::create_private_directory(path)
}

#[cfg(windows)]
pub(crate) fn system_root() -> PathBuf {
    Os::system_root()
}
