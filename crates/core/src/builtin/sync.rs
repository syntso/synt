mod disable;
mod enable;
mod history;
mod params;
mod resolve;
mod share;
mod status;
mod unshare;

use disable::SyncDisable;
use enable::SyncEnable;
use history::DocumentHistory;
use resolve::DocumentResolve;
use share::SyncShare;
use status::SyncStatus;
use unshare::SyncUnshare;

use synt_actions::{ActionError, ActionGroup, Registry};
use synt_sync::Syncer;

#[derive(Clone)]
pub struct SyncActions {
    syncer: Syncer,
}

impl SyncActions {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl ActionGroup for SyncActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(SyncEnable::new(self.syncer.clone()))?;
        registry.register(SyncDisable::new(self.syncer.clone()))?;
        registry.register(SyncStatus::new(self.syncer.clone()))?;
        registry.register(SyncShare::new(self.syncer.clone()))?;
        registry.register(SyncUnshare::new(self.syncer.clone()))?;
        registry.register(DocumentHistory::new(self.syncer.clone()))?;
        registry.register(DocumentResolve::new(self.syncer.clone()))
    }
}
