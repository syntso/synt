use russh_sftp::protocol::FileAttributes;

use super::FileKind;

const PERMISSION_BITS: u32 = 0o7777;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub kind: FileKind,
    pub size: Option<u64>,
    pub mode: Option<u32>,
    pub modified: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

impl FileEntry {
    pub(in crate::sftp) fn new(name: String, attributes: &FileAttributes) -> Self {
        Self {
            name,
            kind: FileKind::from(attributes),
            size: attributes.size,
            mode: attributes.permissions.map(|bits| bits & PERMISSION_BITS),
            modified: attributes.mtime,
            uid: attributes.uid,
            gid: attributes.gid,
        }
    }
}
