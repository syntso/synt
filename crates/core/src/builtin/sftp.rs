mod download;
mod info;
mod list;
mod mkdir;
mod path_params;
mod read;
mod remote_path;
mod remove;
mod rename;
mod stat;
mod upload;
mod write;

pub(crate) use download::SftpDownload;
pub(crate) use info::FileInfo;
pub(crate) use list::SftpList;
pub(crate) use mkdir::SftpMkdir;
pub(crate) use path_params::PathParams;
pub(crate) use read::SftpRead;
pub(crate) use remote_path::RemotePath;
pub(crate) use remove::SftpRemove;
pub(crate) use rename::SftpRename;
pub(crate) use stat::SftpStat;
pub(crate) use upload::SftpUpload;
pub(crate) use write::SftpWrite;

use std::future::Future;
use std::sync::Arc;

use synt_actions::{ActionError, ActionGroup, Context, Registry, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{FileSessions, Pool, Sftp, SshError};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::builtin::ssh::failure;
use crate::projects::Projects;

const CONCURRENT_CALLS: usize = 8;
const CONCURRENT_TRANSFERS: usize = 4;

const LARGEST_READ: u64 = 4 * 1024 * 1024;
const LARGEST_WRITE: u64 = 640 * 1024;

#[derive(Debug, Clone)]
pub struct SftpActions {
    sessions: FileSessions,
    projects: Projects,
    calls: Arc<Semaphore>,
    transfers: Arc<Semaphore>,
}

impl SftpActions {
    pub fn new(pool: Pool, projects: Projects) -> Self {
        Self {
            sessions: FileSessions::new(pool),
            projects,
            calls: Arc::new(Semaphore::new(CONCURRENT_CALLS)),
            transfers: Arc::new(Semaphore::new(CONCURRENT_TRANSFERS)),
        }
    }

    async fn run<T, Operation, Running>(
        &self,
        context: &Context,
        operation: Operation,
    ) -> Result<T, ActionError>
    where
        Operation: Fn(Arc<Sftp>) -> Running,
        Running: Future<Output = Result<T, SshError>>,
    {
        let target = self.projects.reach_host(context).await?;

        let _permit = self
            .calls
            .acquire()
            .await
            .map_err(|_| ActionError::failed("sftp is shutting down"))?;

        self.sessions.with(&target, operation).await.map_err(failed)
    }

    fn per_call_risk(&self, target: Option<&ResourceRef>, declared: Risk) -> Risk {
        self.projects.login_risk(target, declared)
    }

    fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        self.projects.host_destination(target)
    }

    async fn transfer_slot(&self) -> Result<OwnedSemaphorePermit, ActionError> {
        Arc::clone(&self.transfers)
            .acquire_owned()
            .await
            .map_err(|_| ActionError::failed("sftp is shutting down"))
    }
}

impl ActionGroup for SftpActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(SftpList::new(self.clone()))?;
        registry.register(SftpStat::new(self.clone()))?;
        registry.register(SftpRead::new(self.clone()))?;
        registry.register(SftpWrite::new(self.clone()))?;
        registry.register(SftpUpload::new(self.clone()))?;
        registry.register(SftpDownload::new(self.clone()))?;
        registry.register(SftpMkdir::new(self.clone()))?;
        registry.register(SftpRemove::new(self.clone()))?;
        registry.register(SftpRename::new(self.clone()))
    }
}

fn failed(error: SshError) -> ActionError {
    match error {
        error @ (SshError::Sftp { .. } | SshError::SftpLost { .. } | SshError::SftpClosed) => {
            ActionError::Failed(error.to_string())
        }
        SshError::Timeout(limit) => ActionError::Failed(format!(
            "the sftp session did not start within {} s",
            limit.as_secs()
        )),
        other => failure(other),
    }
}
