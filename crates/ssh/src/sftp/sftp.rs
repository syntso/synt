use std::fmt::{self, Debug, Formatter};
use std::io::SeekFrom;
use std::time::Duration;

use russh_sftp::client::SftpSession;
use russh_sftp::client::fs::File;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::time;

use super::{Directory, FileEntry, FileKind, WriteMode, failure, io_failure};
use crate::{Lease, Pool, SshError, Target};

const OPEN_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const SUBSYSTEM: &str = "sftp";
const LONGEST_LISTING: usize = 10_000;

pub struct Sftp {
    session: SftpSession,
    lease: Lease,
}

impl Sftp {
    pub(crate) async fn open(pool: &Pool, target: &Target) -> Result<Self, SshError> {
        time::timeout(OPEN_TIMEOUT, Self::start(pool, target))
            .await
            .map_err(|_| SshError::Timeout(OPEN_TIMEOUT))?
    }

    pub async fn list(&self, path: &str) -> Result<Directory, SshError> {
        let listing = match self.session.read_dir(path).await {
            Ok(listing) => listing,
            Err(error) => return Err(self.explain_list(path, failure("list", path)(error)).await),
        };

        let mut entries: Vec<FileEntry> = listing
            .filter(|entry| !is_dot(&entry.file_name()))
            .map(|entry| FileEntry::new(entry.file_name(), &entry.metadata()))
            .collect();

        entries.sort_by(|left, right| left.name.cmp(&right.name));

        let truncated = entries.len() > LONGEST_LISTING;
        entries.truncate(LONGEST_LISTING);

        Ok(Directory { entries, truncated })
    }

    pub async fn stat(&self, path: &str) -> Result<FileEntry, SshError> {
        let attributes = self
            .session
            .metadata(path)
            .await
            .map_err(failure("stat", path))?;

        Ok(FileEntry::new(path.to_owned(), &attributes))
    }

    pub async fn read(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>, SshError> {
        let mut file = self
            .session
            .open(path)
            .await
            .map_err(failure("read", path))?;

        let read = read_range(&mut file, offset, length).await;
        let closed = file.close().await;

        let data = read.map_err(io_failure("read", path))?;
        closed.map_err(io_failure("read", path))?;

        Ok(data)
    }

    pub async fn write(
        &self,
        path: &str,
        offset: u64,
        data: &[u8],
        mode: WriteMode,
    ) -> Result<(), SshError> {
        let mut file = self
            .session
            .open_with_flags(path, open_flags(mode))
            .await
            .map_err(failure("write", path))?;

        let written = write_range(&mut file, offset, data).await;
        let closed = file.close().await;

        written.map_err(io_failure("write", path))?;
        closed.map_err(io_failure("write", path))
    }

    pub async fn mkdir(&self, path: &str) -> Result<(), SshError> {
        let Err(error) = self.session.create_dir(path).await else {
            return Ok(());
        };

        let error = failure("mkdir", path)(error);

        match self.session.symlink_metadata(path).await {
            Ok(existing) => Err(explained(error, existing_reason(&existing))),
            Err(_) => Err(error),
        }
    }

    pub async fn remove(&self, path: &str) -> Result<FileKind, SshError> {
        let attributes = self
            .session
            .symlink_metadata(path)
            .await
            .map_err(failure("remove", path))?;

        let kind = FileKind::from(attributes.file_type());

        if kind != FileKind::Directory {
            self.session
                .remove_file(path)
                .await
                .map_err(failure("remove", path))?;

            return Ok(kind);
        }

        let Err(error) = self.session.remove_dir(path).await else {
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
            .map_err(failure("rename", from))
    }

    async fn explain_list(&self, path: &str, error: SshError) -> SshError {
        match self.session.metadata(path).await {
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

        let session = SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| SshError::Channel(format!("sftp did not start: {error}")))?;

        session.set_timeout(REQUEST_TIMEOUT_SECONDS);

        Ok(Self { session, lease })
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

async fn read_range(file: &mut File, offset: u64, length: u64) -> std::io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset)).await?;

    let capacity = usize::try_from(length).unwrap_or(0);
    let mut data = Vec::with_capacity(capacity);

    (&mut *file).take(length).read_to_end(&mut data).await?;

    Ok(data)
}

async fn write_range(file: &mut File, offset: u64, data: &[u8]) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(offset)).await?;
    file.write_all(data).await?;

    file.flush().await
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
