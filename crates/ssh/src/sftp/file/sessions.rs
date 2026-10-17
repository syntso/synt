mod slot;

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;

use synt_platform::Lock;
use tokio::time;

use slot::SessionSlot;

use crate::{Pool, Sftp, SshError, Target};

const IDLE: Duration = Duration::from_secs(30);

type Slots = HashMap<Target, Arc<SessionSlot>>;

#[derive(Clone)]
pub struct FileSessions {
    pool: Pool,
    slots: Arc<Mutex<Slots>>,
}

impl FileSessions {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            slots: Arc::new(Mutex::new(Slots::new())),
        }
    }

    pub async fn with<T, Operation, Running>(
        &self,
        target: &Target,
        operation: Operation,
    ) -> Result<T, SshError>
    where
        Operation: Fn(Arc<Sftp>) -> Running,
        Running: Future<Output = Result<T, SshError>>,
    {
        match self.attempt(target, &operation).await {
            Err(SshError::SftpClosed) => self.attempt(target, &operation).await,
            other => other,
        }
    }

    async fn attempt<T, Operation, Running>(
        &self,
        target: &Target,
        operation: &Operation,
    ) -> Result<T, SshError>
    where
        Operation: Fn(Arc<Sftp>) -> Running,
        Running: Future<Output = Result<T, SshError>>,
    {
        let slot = self.slot(target);
        let session = slot.session(&self.pool, target).await?;

        if slot.start_watching() {
            self.watch(target.clone(), Arc::downgrade(&slot));
        }

        let outcome = operation(session).await;
        slot.touch();

        if matches!(
            outcome,
            Err(SshError::SftpClosed | SshError::SftpLost { .. })
        ) {
            self.discard(target, &slot);
        }

        outcome
    }

    fn slot(&self, target: &Target) -> Arc<SessionSlot> {
        let mut slots = self.slots();

        let slot = slots
            .entry(target.clone())
            .or_insert_with(|| Arc::new(SessionSlot::new()));

        Arc::clone(slot)
    }

    fn watch(&self, target: Target, slot: Weak<SessionSlot>) {
        let sessions = self.clone();

        tokio::spawn(async move {
            let mut wait = IDLE;

            loop {
                time::sleep(wait).await;

                let Some(watched) = slot.upgrade() else {
                    return;
                };

                if !sessions.is_current(&target, &watched) {
                    return;
                }

                let Some(left) = watched.idle_left(IDLE) else {
                    sessions.discard(&target, &watched);
                    return;
                };

                wait = left;
            }
        });
    }

    fn slots(&self) -> MutexGuard<'_, Slots> {
        self.slots.locked()
    }

    fn is_current(&self, target: &Target, slot: &Arc<SessionSlot>) -> bool {
        Self::holds(&self.slots(), target, slot)
    }

    fn discard(&self, target: &Target, slot: &Arc<SessionSlot>) {
        let mut slots = self.slots();

        if Self::holds(&slots, target, slot) {
            slots.remove(target);
        }
    }

    fn holds(slots: &Slots, target: &Target, slot: &Arc<SessionSlot>) -> bool {
        slots
            .get(target)
            .is_some_and(|current| Arc::ptr_eq(current, slot))
    }
}

impl Debug for FileSessions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileSessions")
            .field("open", &self.slots().len())
            .finish()
    }
}
