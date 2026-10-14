use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio_util::sync::CancellationToken;

use super::actor_text::actor_text;
use super::holders::Holders;
use super::{GrantId, GrantSeal, Hold, StandingGrant};
use crate::CoreError;
use crate::access::AccessRequest;
use crate::blocking::blocking;
use crate::locked::locked;
use crate::state::{NewGrant, State, StoredGrant};

type Kept = BTreeMap<GrantId, (StoredGrant, StandingGrant)>;

#[derive(Debug, Clone)]
pub struct AlwaysGrants {
    state: State,
    seal: Arc<GrantSeal>,
    kept: Arc<Mutex<Kept>>,
    writing: Arc<Mutex<()>>,
    holders: Arc<Holders>,
}

impl AlwaysGrants {
    pub fn load(state: State, seal: GrantSeal) -> Result<Self, CoreError> {
        let stored = sealed_grants(&state, &seal)?;
        let mut kept = Kept::new();

        for stored in stored {
            let id = stored.id;

            match StandingGrant::from_stored(stored.clone()) {
                Some(grant) => {
                    kept.insert(grant.id, (stored, grant));
                }
                None => eprintln!("synt-core: grant {id} names an unknown actor; it is ignored"),
            }
        }

        Ok(Self {
            state,
            seal: Arc::new(seal),
            kept: Arc::new(Mutex::new(kept)),
            writing: Arc::default(),
            holders: Arc::default(),
        })
    }

    pub fn list(&self) -> Vec<StandingGrant> {
        self.lock()
            .values()
            .map(|(_, grant)| grant.clone())
            .collect()
    }

    pub fn covering(&self, request: &AccessRequest<'_>) -> Option<GrantId> {
        self.lock()
            .values()
            .find(|(_, grant)| grant.covers(request))
            .map(|(_, grant)| grant.id)
    }

    pub async fn give(&self, request: &AccessRequest<'_>) -> Result<GrantId, CoreError> {
        let grant = NewGrant {
            actor: actor_text(request.actor),
            action: request.action.as_str().to_owned(),
            resource: request.target.map(|target| target.as_str().to_owned()),
            destination: request.destination.map(str::to_owned),
        };
        let grants = self.clone();

        blocking(move || grants.keep(grant)).await
    }

    pub async fn revoke(&self, id: GrantId) -> Result<Option<(StandingGrant, usize)>, CoreError> {
        let grants = self.clone();
        let Some(grant) = blocking(move || grants.forget(id)).await? else {
            return Ok(None);
        };

        let stopped = self.holders.stop_all(id);

        Ok(Some((grant, stopped)))
    }

    pub(crate) fn hold(&self, id: GrantId, stop: &CancellationToken) -> Hold {
        let hold = self.holders.hold(id, stop.clone());

        if !self.lock().contains_key(&id) {
            stop.cancel();
        }

        hold
    }

    fn keep(&self, grant: NewGrant) -> Result<GrantId, CoreError> {
        let _writing = locked(&self.writing);

        let stored = self.state.insert_grant(grant)?;
        let standing = StandingGrant::from_stored(stored.clone())
            .ok_or_else(|| CoreError::Grant("the new grant cannot be read back".to_owned()))?;

        let mut all = self.stored();
        all.push(stored.clone());

        if let Err(error) = self.seal.save(&all) {
            self.state.delete_grant(stored.id)?;
            return Err(error);
        }

        let id = standing.id;
        self.lock().insert(id, (stored, standing));

        Ok(id)
    }

    fn forget(&self, id: GrantId) -> Result<Option<StandingGrant>, CoreError> {
        let _writing = locked(&self.writing);

        if !self.lock().contains_key(&id) {
            return Ok(None);
        }

        self.state.delete_grant(id.0)?;
        let removed = self.lock().remove(&id);

        if let Err(error) = self.seal.save(&self.stored()) {
            eprintln!(
                "synt-core: the grants could not be sealed after a revocation, so the next start \
                 drops them all: {error}"
            );
        }

        Ok(removed.map(|(_, grant)| grant))
    }

    fn stored(&self) -> Vec<StoredGrant> {
        self.lock()
            .values()
            .map(|(stored, _)| stored.clone())
            .collect()
    }

    fn lock(&self) -> MutexGuard<'_, Kept> {
        locked(&self.kept)
    }
}

fn sealed_grants(state: &State, seal: &GrantSeal) -> Result<Vec<StoredGrant>, CoreError> {
    let stored = state.grants()?;

    if seal.matches(&stored)? {
        return Ok(stored);
    }

    eprintln!(
        "synt-core: the standing grants were changed outside the core, so all {} of them are \
         dropped; the person will be asked again",
        stored.len()
    );

    state.clear_grants()?;
    seal.save(&[])?;

    Ok(Vec::new())
}
