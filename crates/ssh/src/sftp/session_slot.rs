use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::OnceCell;

use super::Sftp;
use crate::locked::locked;
use crate::{Pool, SshError, Target};

pub(super) struct SessionSlot {
    session: OnceCell<Arc<Sftp>>,
    last_used: Mutex<Instant>,
}

impl SessionSlot {
    pub(super) fn new() -> Self {
        Self {
            session: OnceCell::new(),
            last_used: Mutex::new(Instant::now()),
        }
    }

    pub(super) async fn session(
        &self,
        pool: &Pool,
        target: &Target,
    ) -> Result<(Arc<Sftp>, bool), SshError> {
        let mut opened = false;

        let session = self
            .session
            .get_or_try_init(|| async {
                opened = true;
                Sftp::open(pool, target).await.map(Arc::new)
            })
            .await?;

        self.touch();

        Ok((Arc::clone(session), opened))
    }

    pub(super) fn touch(&self) {
        *locked(&self.last_used) = Instant::now();
    }

    pub(super) fn idle_left(&self, idle: Duration) -> Option<Duration> {
        let in_use = self
            .session
            .get()
            .is_some_and(|session| Arc::strong_count(session) > 1);

        if in_use {
            return Some(idle);
        }

        let last_used = *locked(&self.last_used);

        idle.checked_sub(last_used.elapsed())
            .filter(|left| !left.is_zero())
    }
}
