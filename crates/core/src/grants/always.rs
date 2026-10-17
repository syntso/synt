use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use synt_platform::Lock;
use tokio_util::sync::CancellationToken;

use super::{
    GrantId, GrantScope, GrantSeal, GrantTable, GrantUsers, Hold, Revocation, StandingGrant,
};
use crate::CoreError;
use crate::access::AccessRequest;
use crate::blocking::blocking;
use crate::log::log;
use crate::state::State;

type Kept = BTreeMap<GrantId, StandingGrant>;

#[derive(Debug, Clone)]
pub struct AlwaysGrants {
    table: GrantTable,
    seal: Arc<GrantSeal>,
    kept: Arc<Mutex<Kept>>,
    writing: Arc<Mutex<()>>,
    users: Arc<GrantUsers>,
}

impl AlwaysGrants {
    pub fn load(state: State, seal: GrantSeal) -> Result<Self, CoreError> {
        let table = GrantTable::new(state);
        let sealed = Self::sealed(&table, &seal)?;
        let kept = sealed
            .into_iter()
            .map(|grant| (grant.id(), grant))
            .collect();

        Ok(Self {
            table,
            seal: Arc::new(seal),
            kept: Arc::new(Mutex::new(kept)),
            writing: Arc::default(),
            users: Arc::default(),
        })
    }

    pub fn list(&self) -> Vec<StandingGrant> {
        self.lock().values().cloned().collect()
    }

    pub(crate) fn covering(&self, request: &AccessRequest<'_>) -> Option<GrantId> {
        self.lock()
            .values()
            .find(|grant| grant.scope().covers(request))
            .map(StandingGrant::id)
    }

    pub(crate) async fn give(&self, request: &AccessRequest<'_>) -> Result<GrantId, CoreError> {
        let scope = GrantScope::covering(request);
        let grants = self.clone();

        blocking(move || grants.keep(scope)).await
    }

    pub(crate) async fn revoke(&self, id: GrantId) -> Result<Option<Revocation>, CoreError> {
        let grants = self.clone();

        let Some(grant) = blocking::<_, CoreError>(move || grants.forget(id)).await? else {
            return Ok(None);
        };

        let stopped = self.users.stop_all(id);

        Ok(Some(Revocation { grant, stopped }))
    }

    pub(crate) fn hold(&self, id: GrantId, stop: &CancellationToken) -> Hold {
        let hold = self.users.hold(id, stop.clone());

        if !self.lock().contains_key(&id) {
            stop.cancel();
        }

        hold
    }

    fn sealed(table: &GrantTable, seal: &GrantSeal) -> Result<Vec<StandingGrant>, CoreError> {
        let stored = table.all()?;

        if seal.matches(&stored)? {
            return Ok(stored);
        }

        log!(
            "the standing grants were changed outside the core, so all {} of them are \
             dropped; the person will be asked again",
            stored.len()
        );

        table.clear()?;
        seal.save(&[])?;

        Ok(Vec::new())
    }

    fn keep(&self, scope: GrantScope) -> Result<GrantId, CoreError> {
        let _writing = self.writing.locked();

        let grant = self.table.insert(scope)?;

        let mut all = self.list();
        all.push(grant.clone());

        if let Err(error) = self.seal.save(&all) {
            self.table.delete(grant.id())?;
            return Err(error);
        }

        let id = grant.id();
        self.lock().insert(id, grant);

        Ok(id)
    }

    fn forget(&self, id: GrantId) -> Result<Option<StandingGrant>, CoreError> {
        let _writing = self.writing.locked();

        if !self.lock().contains_key(&id) {
            return Ok(None);
        }

        self.table.delete(id)?;
        let removed = self.lock().remove(&id);

        if let Err(error) = self.seal.save(&self.list()) {
            log!(
                "the grants could not be sealed after a revocation, so the next start \
                 drops them all: {error}"
            );
        }

        Ok(removed)
    }

    fn lock(&self) -> MutexGuard<'_, Kept> {
        self.kept.locked()
    }
}
