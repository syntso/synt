use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use synt_platform::Lock;
use tokio::sync::OnceCell;

use crate::{Pool, Sftp, SshError, Target};

pub(super) struct SessionSlot {
    session: OnceCell<Arc<Sftp>>,
    last_used: Mutex<Instant>,
    watched: AtomicBool,
}

impl SessionSlot {
    pub(super) fn new() -> Self {
        Self {
            session: OnceCell::new(),
            last_used: Mutex::new(Instant::now()),
            watched: AtomicBool::new(false),
        }
    }

    pub(super) async fn session(&self, pool: &Pool, target: &Target) -> Result<Arc<Sftp>, SshError> {
        let session = self
            .session
            .get_or_try_init(|| async { Sftp::open(pool, target).await.map(Arc::new) })
            .await?;

        self.touch();

        Ok(Arc::clone(session))
    }

    pub(super) fn start_watching(&self) -> bool {
        !self.watched.swap(true, Ordering::AcqRel)
    }

    pub(super) fn touch(&self) {
        *self.last_used.locked() = Instant::now();
    }

    pub(super) fn idle_left(&self, idle: Duration) -> Option<Duration> {
        let in_use = self
            .session
            .get()
            .is_some_and(|session| Arc::strong_count(session) > 1);

        if in_use {
            return Some(idle);
        }

        let last_used = *self.last_used.locked();

        idle.checked_sub(last_used.elapsed())
            .filter(|left| !left.is_zero())
    }
}
