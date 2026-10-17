mod dial_attempt;
mod holders;
mod phase;

use std::mem;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use synt_platform::Lock;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time;

use dial_attempt::DialAttempt;
use holders::Holders;
use phase::Phase;

use super::{ConnectionStatus, Lease, Registry};
use crate::session::Connection;
use crate::{Algorithms, SshError, Target};

const RETRY_PAUSE: Duration = Duration::from_secs(3);

pub(crate) struct Slot {
    target: Target,
    pool: Weak<Registry>,
    phase: Mutex<Phase>,
    holders: Mutex<Holders>,
    upstream: Mutex<Option<Lease>>,
    dialing: AsyncMutex<()>,
}

impl Slot {
    pub(crate) fn new(target: Target, pool: Weak<Registry>) -> Self {
        Self {
            target,
            pool,
            phase: Mutex::new(Phase::Idle),
            holders: Mutex::new(Holders::default()),
            upstream: Mutex::new(None),
            dialing: AsyncMutex::new(()),
        }
    }

    pub(crate) fn target(&self) -> &Target {
        &self.target
    }

    pub(crate) fn has_failed(&self) -> bool {
        self.phase.locked().error().is_some()
    }

    pub(crate) fn status(&self) -> ConnectionStatus {
        let phase = self.phase.locked();

        ConnectionStatus {
            target: self.target.label(),
            via: self.target.previous_hop().map(|hop| hop.label()),
            state: phase.state(),
            leases: self.holders.locked().count(),
            error: phase.error().map(ToString::to_string),
        }
    }

    pub(crate) fn algorithms(&self) -> Option<Algorithms> {
        self.phase.locked().connection()?.algorithms()
    }

    pub(crate) fn join(&self) {
        self.holders.locked().join();
    }

    pub(crate) fn leave(&self) -> Option<u64> {
        self.holders.locked().leave()
    }

    pub(crate) fn idle_since(&self, release: u64) -> bool {
        self.holders.locked().idle_since(release)
    }

    pub(crate) async fn connection(&self) -> Result<Connection, SshError> {
        if let Some(ready) = self.ready()? {
            return Ok(ready);
        }

        let _turn = self.dialing.lock().await;

        if let Some(ready) = self.ready()? {
            return Ok(ready);
        }

        self.dial().await
    }

    pub(crate) async fn expire(self: Arc<Self>, release: u64) {
        let Some(pool) = self.pool.upgrade() else {
            return;
        };

        time::sleep(pool.grace()).await;

        if pool.retire(&self, release) {
            self.shut_down().await;
        }
    }

    pub(crate) fn abandon_dial(&self) {
        let mut phase = self.phase.locked();
        let resting = phase.abandoned();

        *phase = resting;
        drop(phase);

        self.release_upstream();
    }

    fn ready(&self) -> Result<Option<Connection>, SshError> {
        let mut phase = self.phase.locked();

        match &*phase {
            Phase::Connected(connection) if connection.is_closed() => {
                *phase = Phase::Lost;
                Ok(None)
            }
            Phase::Connected(connection) => Ok(Some(connection.clone())),
            Phase::Failed(error, at) if at.elapsed() < RETRY_PAUSE => Err(error.clone()),
            Phase::Failed(..)
            | Phase::Idle
            | Phase::Connecting
            | Phase::Lost
            | Phase::Reconnecting => Ok(None),
        }
    }

    async fn dial(&self) -> Result<Connection, SshError> {
        self.begin_dialing();

        let attempt = DialAttempt::new(self);
        let opened = self.open().await;
        attempt.settle();

        match opened {
            Ok(connection) => {
                self.enter(Phase::Connected(connection.clone()));
                Ok(connection)
            }
            Err(error) => {
                self.enter(Phase::Failed(error.clone(), Instant::now()));
                self.release_upstream();
                Err(error)
            }
        }
    }

    async fn open(&self) -> Result<Connection, SshError> {
        let pool = self.pool.upgrade().ok_or(SshError::PoolClosed)?;
        let through = self.reach_upstream(&pool).await?;

        pool.connector().open(&self.target, through.as_ref()).await
    }

    async fn reach_upstream(&self, pool: &Arc<Registry>) -> Result<Option<Connection>, SshError> {
        let Some(previous) = self.target.previous_hop() else {
            return Ok(None);
        };

        let lease = pool.lease(&previous);
        let connection = lease.connection().await?;

        let replaced = self.upstream.locked().replace(lease);
        drop(replaced);

        Ok(Some(connection))
    }

    async fn shut_down(&self) {
        let _turn = self.dialing.lock().await;

        let previous = mem::replace(&mut *self.phase.locked(), Phase::Idle);

        if let Phase::Connected(connection) = previous {
            connection.close().await;
        }

        self.release_upstream();
    }

    fn begin_dialing(&self) {
        let mut phase = self.phase.locked();
        let next = phase.dialing();

        *phase = next;
    }

    fn enter(&self, next: Phase) {
        let previous = mem::replace(&mut *self.phase.locked(), next);
        drop(previous);
    }

    fn release_upstream(&self) {
        let upstream = self.upstream.locked().take();
        drop(upstream);
    }
}
