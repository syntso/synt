use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use synt_platform::{read_plain_file, replace_file};

use super::RevisionFile;
use crate::SyncError;
use crate::id::{MemberId, ProjectId};
use crate::provider::{Cursor, MemberKey, SealedRevision, SyncProvider, WrappedKey};

const MEMBERS: &str = "members";
const KEYS: &str = "keys";
const REVISIONS: &str = "revisions";
const JSON: &str = "json";
const LARGEST_FILE: u64 = 4 * 1024 * 1024;
const LATE_WRITERS_MS: u64 = 60_000;

#[derive(Debug, Clone)]
pub struct FolderProvider {
    root: PathBuf,
}

impl FolderProvider {
    pub fn open(root: &Path) -> Result<Self, SyncError> {
        if !root.is_dir() {
            return Err(SyncError::provider(format!(
                "the sync folder {} does not exist",
                root.display()
            )));
        }

        Ok(Self {
            root: root.to_owned(),
        })
    }

    fn folder(&self, project: &ProjectId, part: &str) -> PathBuf {
        self.root.join(project.get()).join(part)
    }

    fn member_file(&self, project: &ProjectId, part: &str, member: &MemberId) -> PathBuf {
        self.folder(project, part).join(format!("{member}.{JSON}"))
    }

    fn revision_files(&self, project: &ProjectId) -> Result<Vec<RevisionFile>, SyncError> {
        let mut files: Vec<RevisionFile> = names(&self.folder(project, REVISIONS))?
            .iter()
            .filter_map(|name| RevisionFile::parse(name))
            .collect();
        files.sort();

        Ok(files)
    }
}

impl SyncProvider for FolderProvider {
    fn push(&self, project: &ProjectId, revisions: &[SealedRevision]) -> Result<(), SyncError> {
        let folder = self.folder(project, REVISIONS);

        for revision in revisions {
            let file = RevisionFile::new(revision.at, &revision.revision);

            write_json(&folder.join(file.name()), revision)?;
        }

        Ok(())
    }

    fn pull(
        &self,
        project: &ProjectId,
        since: &Cursor,
    ) -> Result<(Vec<SealedRevision>, Cursor), SyncError> {
        let floor = RevisionFile::parse(since.get())
            .map(|seen| RevisionFile::floor(seen.at().saturating_sub(LATE_WRITERS_MS)))
            .unwrap_or_default();
        let folder = self.folder(project, REVISIONS);

        let mut revisions = Vec::new();
        let mut cursor = since.clone();

        for file in self.revision_files(project)? {
            if file.name() < floor.as_str() {
                continue;
            }

            if let Some(revision) = read_json(&folder.join(file.name()))? {
                revisions.push(revision);
            }

            if file.name() > cursor.get() {
                cursor = Cursor::new(file.name());
            }
        }

        Ok((revisions, cursor))
    }

    fn members(&self, project: &ProjectId) -> Result<Vec<MemberKey>, SyncError> {
        let folder = self.folder(project, MEMBERS);
        let mut members = Vec::new();

        for name in names(&folder)? {
            let Some(member) = name
                .strip_suffix(".json")
                .and_then(|id| MemberId::parse(id).ok())
            else {
                continue;
            };

            let Some(key) = read_json::<MemberKey>(&folder.join(&name))? else {
                continue;
            };

            if key.member == member && key.check().is_ok() {
                members.push(key);
            }
        }

        members.sort_by(|one, other| one.member.cmp(&other.member));

        Ok(members)
    }

    fn join(&self, project: &ProjectId, member: &MemberKey) -> Result<(), SyncError> {
        write_json(&self.member_file(project, MEMBERS, &member.member), member)
    }

    fn leave(&self, project: &ProjectId, member: &MemberId) -> Result<(), SyncError> {
        remove(&self.member_file(project, KEYS, member))?;
        remove(&self.member_file(project, MEMBERS, member))
    }

    fn share(&self, project: &ProjectId, wrapped: &[WrappedKey]) -> Result<(), SyncError> {
        for key in wrapped {
            write_json(&self.member_file(project, KEYS, &key.member), key)?;
        }

        Ok(())
    }

    fn key(&self, project: &ProjectId, member: &MemberId) -> Result<Option<WrappedKey>, SyncError> {
        let key: Option<WrappedKey> = read_json(&self.member_file(project, KEYS, member))?;

        Ok(key.filter(|key| &key.member == member))
    }
}

fn names(folder: &Path) -> Result<Vec<String>, SyncError> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(failed(folder, &error)),
    };

    let mut names = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| failed(folder, &error))?;

        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_owned());
        }
    }

    Ok(names)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, SyncError> {
    let bytes = match read_plain_file(path, LARGEST_FILE) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failed(path, &error)),
    };

    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| SyncError::provider(format!("{} is damaged: {error}", path.display())))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), SyncError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| SyncError::provider(format!("cannot encode: {error}")))?;

    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| failed(folder, &error))?;
    }

    replace_file(path, &bytes).map_err(|error| failed(path, &error))
}

fn remove(path: &Path) -> Result<(), SyncError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(failed(path, &error)),
        _ => Ok(()),
    }
}

fn failed(path: &Path, error: &io::Error) -> SyncError {
    SyncError::provider(format!("{}: {error}", path.display()))
}
