use std::io;
use std::path::PathBuf;

const FOLDER: &str = ".ssh";

#[derive(Debug)]
pub(crate) enum SshDirectory {
    User,
    At(PathBuf),
}

impl SshDirectory {
    pub(crate) fn path(&self) -> io::Result<PathBuf> {
        match self {
            Self::User => Ok(synt_platform::home_directory()?.join(FOLDER)),
            Self::At(path) => Ok(path.clone()),
        }
    }
}
