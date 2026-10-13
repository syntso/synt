use std::ffi::OsStr;
use std::fs::{self, DirBuilder, File, OpenOptions, Permissions, TryLockError};
use std::io;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use tokio::net::UnixListener;

use super::Stream;

const PRIVATE_DIRECTORY: u32 = 0o700;
const PRIVATE_SOCKET: u32 = 0o600;
const SHARED_BITS: u32 = 0o077;

pub(crate) struct Listener {
    socket: UnixListener,
    path: PathBuf,
    _lock: File,
}

impl Listener {
    pub(crate) fn bind(address: &OsStr) -> io::Result<Self> {
        let path = PathBuf::from(address);
        let lock = lock_beside(&path)?;

        clear_stale_socket(&path)?;

        let socket = UnixListener::bind(&path)?;
        fs::set_permissions(&path, Permissions::from_mode(PRIVATE_SOCKET))?;

        Ok(Self {
            socket,
            path,
            _lock: lock,
        })
    }

    pub(crate) async fn accept(&mut self) -> io::Result<Stream> {
        let (stream, _) = self.socket.accept().await?;
        Ok(stream)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn lock_beside(socket: &Path) -> io::Result<File> {
    let directory = socket.parent().unwrap_or(Path::new("."));
    DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_DIRECTORY)
        .create(directory)?;

    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock_path(socket))?;

    make_private(directory, &lock)?;

    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(TryLockError::WouldBlock) => Err(already_listening()),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

fn make_private(directory: &Path, ours: &File) -> io::Result<()> {
    let metadata = fs::metadata(directory)?;
    let user = ours.metadata()?.uid();

    if metadata.uid() != user {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the endpoint directory belongs to another user",
        ));
    }
    if metadata.mode() & SHARED_BITS == 0 {
        return Ok(());
    }

    fs::set_permissions(directory, Permissions::from_mode(PRIVATE_DIRECTORY))
}

fn clear_stale_socket(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    if !metadata.file_type().is_socket() {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "the endpoint path is taken by something other than a socket",
        ));
    }
    if UnixStream::connect(path).is_ok() {
        return Err(already_listening());
    }

    fs::remove_file(path)
}

fn lock_path(socket: &Path) -> PathBuf {
    let mut path = socket.as_os_str().to_owned();
    path.push(".lock");

    PathBuf::from(path)
}

fn already_listening() -> io::Error {
    io::Error::new(
        io::ErrorKind::AddrInUse,
        "another core is already listening",
    )
}
