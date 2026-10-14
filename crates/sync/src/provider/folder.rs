mod at_path;
mod json_file;
mod revision_file;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::SyncError;
use crate::id::{MemberId, ProjectId};
use crate::provider::{Cursor, MemberKey, SealedRevision, SyncProvider, WrappedKey};
use at_path::AtPath;
use json_file::JsonFile;
use revision_file::RevisionFile;

const MEMBERS: &str = "members";
const KEYS: &str = "keys";
const REVISIONS: &str = "revisions";
const JSON_SUFFIX: &str = ".json";
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

    fn member_file(&self, project: &ProjectId, part: &str, member: &MemberId) -> JsonFile {
        JsonFile::new(
            self.folder(project, part)
                .join(format!("{member}{JSON_SUFFIX}")),
        )
    }

    fn revision_files(&self, project: &ProjectId) -> Result<Vec<RevisionFile>, SyncError> {
        let mut files: Vec<RevisionFile> = Self::names(&self.folder(project, REVISIONS))?
            .iter()
            .filter_map(|name| RevisionFile::parse(name))
            .collect();
        files.sort();

        Ok(files)
    }

    fn member(folder: &Path, name: &str) -> Result<Option<MemberKey>, SyncError> {
        let Some(member) = name
            .strip_suffix(JSON_SUFFIX)
            .and_then(|id| MemberId::parse(id).ok())
        else {
            return Ok(None);
        };

        let key: Option<MemberKey> = JsonFile::new(folder.join(name)).read()?;

        Ok(key.filter(|key| key.member == member && key.check().is_ok()))
    }

    fn names(folder: &Path) -> Result<Vec<String>, SyncError> {
        let entries = match fs::read_dir(folder) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            read => read.at(folder)?,
        };

        let mut names = Vec::new();

        for entry in entries {
            if let Some(name) = entry.at(folder)?.file_name().to_str() {
                names.push(name.to_owned());
            }
        }

        Ok(names)
    }
}

impl SyncProvider for FolderProvider {
    fn push(&self, project: &ProjectId, revisions: &[SealedRevision]) -> Result<(), SyncError> {
        let folder = self.folder(project, REVISIONS);

        for revision in revisions {
            let file = RevisionFile::new(revision.at, &revision.revision);

            JsonFile::new(folder.join(file.name())).write(revision)?;
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

            if let Some(revision) = JsonFile::new(folder.join(file.name())).read()? {
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

        for name in Self::names(&folder)? {
            members.extend(Self::member(&folder, &name)?);
        }

        members.sort_by(|one, other| one.member.cmp(&other.member));

        Ok(members)
    }

    fn join(&self, project: &ProjectId, member: &MemberKey) -> Result<(), SyncError> {
        self.member_file(project, MEMBERS, &member.member)
            .write(member)
    }

    fn leave(&self, project: &ProjectId, member: &MemberId) -> Result<(), SyncError> {
        self.member_file(project, KEYS, member).remove()?;
        self.member_file(project, MEMBERS, member).remove()
    }

    fn share(&self, project: &ProjectId, wrapped: &[WrappedKey]) -> Result<(), SyncError> {
        for key in wrapped {
            self.member_file(project, KEYS, &key.member).write(key)?;
        }

        Ok(())
    }

    fn key(&self, project: &ProjectId, member: &MemberId) -> Result<Option<WrappedKey>, SyncError> {
        let key: Option<WrappedKey> = self.member_file(project, KEYS, member).read()?;

        Ok(key.filter(|key| &key.member == member))
    }
}
