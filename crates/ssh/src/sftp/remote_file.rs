use std::sync::Arc;

use russh_sftp::client::RawSftpSession;
use russh_sftp::client::error::Error;
use tokio::runtime::Handle;

pub(super) struct RemoteFile {
    session: Arc<RawSftpSession>,
    handle: String,
    path: String,
    open: bool,
}

impl RemoteFile {
    pub(super) fn new(session: Arc<RawSftpSession>, handle: String, path: &str) -> Self {
        Self {
            session,
            handle,
            path: path.to_owned(),
            open: true,
        }
    }

    pub(super) fn session(&self) -> &Arc<RawSftpSession> {
        &self.session
    }

    pub(super) fn handle(&self) -> &str {
        &self.handle
    }

    pub(super) fn path(&self) -> &str {
        &self.path
    }

    pub(super) async fn sync_and_close(mut self, sync: bool) -> Result<(), Error> {
        self.open = false;

        if !sync {
            return self.session.close(self.handle.as_str()).await.map(|_| ());
        }

        let (synced, closed) = tokio::join!(
            biased;
            self.session.fsync(self.handle.as_str()),
            self.session.close(self.handle.as_str()),
        );

        synced?;
        closed.map(|_| ())
    }
}

impl Drop for RemoteFile {
    fn drop(&mut self) {
        if !self.open {
            return;
        }

        let Ok(runtime) = Handle::try_current() else {
            return;
        };

        let session = Arc::clone(&self.session);
        let handle = std::mem::take(&mut self.handle);

        runtime.spawn(async move {
            let _ = session.close(handle).await;
        });
    }
}
