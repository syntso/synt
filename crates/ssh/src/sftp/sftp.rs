use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

use russh_sftp::client::RawSftpSession;
use russh_sftp::client::error::Error;
use russh_sftp::extensions::FSYNC;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use tokio::time;

use super::{
    Directory, Download, FileEntry, FileKind, RemoteFile, Upload, WriteMode, failure, is_eof,
};
use crate::{Lease, Pool, SshError, Target};

const OPEN_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const SUBSYSTEM: &str = "sftp";
const LONGEST_LISTING: usize = 10_000;
const EXTENSION_VERSION: &str = "1";

pub struct Sftp {
    session: Arc<RawSftpSession>,
    fsync: bool,
    lease: Lease,
}

impl Sftp {
    pub(crate) async fn open(pool: &Pool, target: &Target) -> Result<Self, SshError> {
        time::timeout(OPEN_TIMEOUT, Self::start(pool, target))
            .await
            .map_err(|_| SshError::Timeout(OPEN_TIMEOUT))?
    }

    pub async fn list(&self, path: &str) -> Result<Directory, SshError> {
        let mut entries = match self.read_directory(path).await {
            Ok(entries) => entries,
            Err(error) => return Err(self.explain_list(path, failure("list", path)(error)).await),
        };

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        let truncated = entries.len() > LONGEST_LISTING;
        entries.truncate(LONGEST_LISTING);

        Ok(Directory { entries, truncated })
    }

    pub async fn stat(&self, path: &str) -> Result<FileEntry, SshError> {
        let attributes = self.attributes(path).await.map_err(failure("stat", path))?;

        Ok(FileEntry::new(path.to_owned(), &attributes))
    }

    pub async fn read(
        self: &Arc<Self>,
        path: &str,
        offset: u64,
        length: u64,
    ) -> Result<Vec<u8>, SshError> {
        let mut download = self.download(path, offset, Some(length)).await?;
        let mut data = Vec::with_capacity(usize::try_from(length).unwrap_or(0));

        while let Some(piece) = download.next().await? {
            data.extend_from_slice(&piece);
        }

        Ok(data)
    }

    pub async fn write(
        self: &Arc<Self>,
        path: &str,
        offset: u64,
        data: &[u8],
        mode: WriteMode,
    ) -> Result<(), SshError> {
        let mut upload = self.upload(path, offset, mode, false).await?;

        upload.write(data).await?;
        upload.finish().await
    }

    pub async fn download(
        self: &Arc<Self>,
        path: &str,
        offset: u64,
        length: Option<u64>,
    ) -> Result<Download, SshError> {
        let file = self
            .open_file(path, OpenFlags::READ)
            .await
            .map_err(failure("read", path))?;

        Ok(Download::new(Arc::clone(self), file, offset, length))
    }

    pub async fn upload(
        self: &Arc<Self>,
        path: &str,
        offset: u64,
        mode: WriteMode,
        durable: bool,
    ) -> Result<Upload, SshError> {
        let file = self
            .open_file(path, open_flags(mode))
            .await
            .map_err(failure("write", path))?;

        Ok(Upload::new(
            Arc::clone(self),
            file,
            offset,
            durable && self.fsync,
        ))
    }

    pub async fn mkdir(&self, path: &str) -> Result<(), SshError> {
        let Err(error) = self.session.mkdir(path, FileAttributes::empty()).await else {
            return Ok(());
        };

        let error = failure("mkdir", path)(error);

        match self.session.lstat(path).await {
            Ok(existing) => Err(explained(error, existing_reason(&existing.attrs))),
            Err(_) => Err(error),
        }
    }

    pub async fn remove(&self, path: &str) -> Result<FileKind, SshError> {
        let attributes = self
            .session
            .lstat(path)
            .await
            .map_err(failure("remove", path))?
            .attrs;

        let kind = FileKind::from(attributes.file_type());

        if kind != FileKind::Directory {
            self.session
                .remove(path)
                .await
                .map_err(failure("remove", path))?;

            return Ok(kind);
        }

        let Err(error) = self.session.rmdir(path).await else {
            return Ok(kind);
        };

        let error = failure("remove", path)(error);

        match self.list(path).await {
            Ok(directory) if directory.truncated => Err(explained(
                error,
                format!("the directory is not empty (over {LONGEST_LISTING} entries)"),
            )),
            Ok(directory) if !directory.entries.is_empty() => Err(explained(
                error,
                format!(
                    "the directory is not empty ({} entries)",
                    directory.entries.len()
                ),
            )),
            _ => Err(error),
        }
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<(), SshError> {
        self.session
            .rename(from, to)
            .await
            .map(|_| ())
            .map_err(failure("rename", from))
    }

    async fn attributes(&self, path: &str) -> Result<FileAttributes, Error> {
        Ok(self.session.stat(path).await?.attrs)
    }

    async fn open_file(&self, path: &str, flags: OpenFlags) -> Result<RemoteFile, Error> {
        let handle = self
            .session
            .open(path, flags, FileAttributes::empty())
            .await?
            .handle;

        Ok(RemoteFile::new(Arc::clone(&self.session), handle, path))
    }

    async fn read_directory(&self, path: &str) -> Result<Vec<FileEntry>, Error> {
        let handle = self.session.opendir(path).await?.handle;
        let directory = RemoteFile::new(Arc::clone(&self.session), handle, path);

        let mut entries = Vec::new();

        while entries.len() <= LONGEST_LISTING {
            let names = match self.session.readdir(directory.handle()).await {
                Ok(names) => names,
                Err(error) if is_eof(&error) => break,
                Err(error) => return Err(error),
            };

            let found = names
                .files
                .into_iter()
                .filter(|file| !is_dot(&file.filename))
                .map(|file| FileEntry::new(file.filename, &file.attrs));

            entries.extend(found);
        }

        Ok(entries)
    }

    async fn explain_list(&self, path: &str, error: SshError) -> SshError {
        match self.attributes(path).await {
            Ok(found) if !found.is_dir() => {
                let kind = FileKind::from(found.file_type());
                explained(error, format!("it is a {}, not a directory", kind.name()))
            }
            _ => error,
        }
    }

    async fn start(pool: &Pool, target: &Target) -> Result<Self, SshError> {
        let lease = pool.acquire(target).await?;
        let channel = lease.open_session().await?;

        channel
            .request_subsystem(true, SUBSYSTEM)
            .await
            .map_err(|error| SshError::Channel(error.to_string()))?;

        let session = RawSftpSession::new(channel.into_stream());
        session.set_timeout(REQUEST_TIMEOUT_SECONDS);

        let version = session
            .init()
            .await
            .map_err(|error| SshError::Channel(format!("sftp did not start: {error}")))?;

        let fsync = version
            .extensions
            .get(FSYNC)
            .is_some_and(|supported| supported == EXTENSION_VERSION);

        Ok(Self {
            session: Arc::new(session),
            fsync,
            lease,
        })
    }
}

impl Debug for Sftp {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sftp")
            .field("target", &self.lease.target().label())
            .finish_non_exhaustive()
    }
}

fn explained(error: SshError, reason: String) -> SshError {
    match error {
        SshError::Sftp {
            operation, path, ..
        } => SshError::Sftp {
            operation,
            path,
            reason,
        },
        other => other,
    }
}

fn existing_reason(existing: &FileAttributes) -> String {
    let kind = FileKind::from(existing.file_type());

    format!("a {} with this name already exists", kind.name())
}

fn open_flags(mode: WriteMode) -> OpenFlags {
    let mut flags = OpenFlags::WRITE;

    if mode.create {
        flags |= OpenFlags::CREATE;
    }

    if mode.truncate {
        flags |= OpenFlags::TRUNCATE;
    }

    flags
}

fn is_dot(name: &str) -> bool {
    name == "." || name == ".."
}
