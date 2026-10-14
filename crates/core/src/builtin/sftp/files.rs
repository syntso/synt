use std::future::Future;
use std::sync::Arc;

use synt_actions::{ActionError, Context, Registry, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{FileSessions, Pool, Sftp, SshError};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::{
    SftpDownload, SftpList, SftpMkdir, SftpRead, SftpRemove, SftpRename, SftpStat, SftpUpload,
    SftpWrite,
};
use crate::builtin::ssh::{destination, failure, login_risk, reach_host};
use crate::link::Link;
use crate::projects::Projects;

const CONCURRENT_CALLS: usize = 8;
const CONCURRENT_TRANSFERS: usize = 4;

pub(super) const LARGEST_READ: u64 = 4 * 1024 * 1024;
pub(super) const LARGEST_WRITE: u64 = 640 * 1024;

#[derive(Debug, Clone)]
pub struct Files {
    sessions: FileSessions,
    projects: Projects,
    calls: Arc<Semaphore>,
    transfers: Arc<Semaphore>,
}

impl Files {
    pub fn new(pool: Pool, projects: Projects) -> Self {
        Self {
            sessions: FileSessions::new(pool),
            projects,
            calls: Arc::new(Semaphore::new(CONCURRENT_CALLS)),
            transfers: Arc::new(Semaphore::new(CONCURRENT_TRANSFERS)),
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
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

    pub(super) async fn run<T, Operation, Running>(
        &self,
        context: &Context,
        operation: Operation,
    ) -> Result<T, ActionError>
    where
        Operation: Fn(Arc<Sftp>) -> Running,
        Running: Future<Output = Result<T, SshError>>,
    {
        let target = reach_host(&self.projects, context).await?;

        let _permit = self
            .calls
            .acquire()
            .await
            .map_err(|_| ActionError::failed("sftp is shutting down"))?;

        self.sessions
            .with(&target, operation)
            .await
            .map_err(|error| failed(&error))
    }

    pub(super) fn per_call_risk(&self, target: Option<&ResourceRef>, declared: Risk) -> Risk {
        login_risk(&self.projects, target, declared)
    }

    pub(super) fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        destination(&self.projects, target)
    }

    pub(super) async fn transfer_slot(&self) -> Result<OwnedSemaphorePermit, ActionError> {
        Arc::clone(&self.transfers)
            .acquire_owned()
            .await
            .map_err(|_| ActionError::failed("sftp is shutting down"))
    }
}

pub(super) fn failed(error: &SshError) -> ActionError {
    match error {
        SshError::Sftp { .. } | SshError::SftpLost { .. } | SshError::SftpClosed => {
            ActionError::Failed(error.to_string())
        }
        SshError::Timeout(limit) => ActionError::Failed(format!(
            "the sftp session did not start within {} s",
            limit.as_secs()
        )),
        other => failure(other),
    }
}

pub(super) fn link(context: &Context) -> Result<Arc<Link>, ActionError> {
    context
        .connection::<Link>()
        .ok_or_else(|| ActionError::failed("this call has no connection to carry the file"))
}
