use serde::Serialize;
use synt_ssh::FileEntry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct FileInfo {
    pub name: String,
    pub kind: &'static str,
    pub size: Option<u64>,
    pub mode: Option<String>,
    pub modified: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

impl From<FileEntry> for FileInfo {
    fn from(entry: FileEntry) -> Self {
        Self {
            name: entry.name,
            kind: entry.kind.name(),
            size: entry.size,
            mode: entry.mode.map(|mode| format!("{mode:04o}")),
            modified: entry.modified,
            uid: entry.uid,
            gid: entry.gid,
        }
    }
}
