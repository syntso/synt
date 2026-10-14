use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;

use tokio::time;

use super::{SessionSlot, Sftp};
use crate::locked::locked;
use crate::{Pool, SshError, Target};

const IDLE: Duration = Duration::from_secs(30);

type Slots = HashMap<Target, Arc<SessionSlot>>;

#[derive(Clone)]
pub struct FileSessions {
    shared: Arc<Shared>,
}

struct Shared {
    pool: Pool,
    slots: Mutex<Slots>,
}

impl FileSessions {
    pub fn new(pool: Pool) -> Self {
        let shared = Shared {
            pool,
            slots: Mutex::new(Slots::new()),
        };

        Self {
            shared: Arc::new(shared),
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
        let (session, opened) = slot.session(&self.shared.pool, target).await?;

        if opened {
            self.watch(target.clone(), Arc::downgrade(&slot));
        }

        let outcome = operation(session).await;
        slot.touch();

        if matches!(
            outcome,
            Err(SshError::SftpClosed | SshError::SftpLost { .. })
        ) {
            self.shared.discard(target, &slot);
        }

        outcome
    }

    fn slot(&self, target: &Target) -> Arc<SessionSlot> {
        let mut slots = self.shared.slots();

        let slot = slots
            .entry(target.clone())
            .or_insert_with(|| Arc::new(SessionSlot::new()));

        Arc::clone(slot)
    }

    fn watch(&self, target: Target, slot: Weak<SessionSlot>) {
        let shared = Arc::clone(&self.shared);

        tokio::spawn(async move {
            let mut wait = IDLE;

            loop {
                time::sleep(wait).await;

                let Some(watched) = slot.upgrade() else {
                    return;
                };

                if !shared.is_current(&target, &watched) {
                    return;
                }

                let Some(left) = watched.idle_left(IDLE) else {
                    shared.discard(&target, &watched);
                    return;
                };

                wait = left;
            }
        });
    }
}

impl Shared {
    fn slots(&self) -> MutexGuard<'_, Slots> {
        locked(&self.slots)
    }

    fn is_current(&self, target: &Target, slot: &Arc<SessionSlot>) -> bool {
        holds(&self.slots(), target, slot)
    }

    fn discard(&self, target: &Target, slot: &Arc<SessionSlot>) {
        let mut slots = self.slots();

        if holds(&slots, target, slot) {
            slots.remove(target);
        }
    }
}

fn holds(slots: &Slots, target: &Target, slot: &Arc<SessionSlot>) -> bool {
    slots
        .get(target)
        .is_some_and(|current| Arc::ptr_eq(current, slot))
}

impl Debug for FileSessions {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileSessions")
            .field("open", &self.shared.slots().len())
            .finish()
    }
}
