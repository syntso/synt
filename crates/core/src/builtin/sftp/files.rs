use std::future::Future;
use std::sync::Arc;

use synt_actions::{ActionError, Context, Registry, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{FileSessions, Pool, Sftp, SshError};
use tokio::sync::Semaphore;

use super::{SftpList, SftpMkdir, SftpRead, SftpRemove, SftpRename, SftpStat, SftpWrite};
use crate::builtin::ssh::{failure, hidden_root, is_root, logs_in_as_root, resolve};

const CONCURRENT_CALLS: usize = 8;

pub(super) const LARGEST_READ: u64 = 4 * 1024 * 1024;
pub(super) const LARGEST_WRITE: u64 = 640 * 1024;

#[derive(Debug, Clone)]
pub struct Files {
    sessions: FileSessions,
    calls: Arc<Semaphore>,
}

impl Files {
    pub fn new(pool: Pool) -> Self {
        Self {
            sessions: FileSessions::new(pool),
            calls: Arc::new(Semaphore::new(CONCURRENT_CALLS)),
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(SftpList::new(self.clone()))?;
        registry.register(SftpStat::new(self.clone()))?;
        registry.register(SftpRead::new(self.clone()))?;
        registry.register(SftpWrite::new(self.clone()))?;
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
        let host = context.target().ok_or(ActionError::TargetRequired)?;
        let target = resolve(host.as_str()).await?;

        if is_root(target.user()) && context.risk() != Risk::Privileged {
            return Err(hidden_root(host));
        }

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
}

pub(super) fn per_call_risk(target: Option<&ResourceRef>, declared: Risk) -> Risk {
    if logs_in_as_root(target) {
        return Risk::Privileged;
    }

    declared
}

fn failed(error: &SshError) -> ActionError {
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
