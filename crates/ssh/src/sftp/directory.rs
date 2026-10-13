use super::FileEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub entries: Vec<FileEntry>,
    pub truncated: bool,
}
