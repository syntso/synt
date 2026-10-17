use russh_sftp::protocol::{FileAttributes, FileType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Directory,
    Symlink,
    Other,
}

impl FileKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }
}

impl From<&FileAttributes> for FileKind {
    fn from(attributes: &FileAttributes) -> Self {
        match attributes.file_type() {
            FileType::File => Self::File,
            FileType::Dir => Self::Directory,
            FileType::Symlink => Self::Symlink,
            FileType::Other => Self::Other,
        }
    }
}
