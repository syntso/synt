use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use synt_actions::Context;
use tokio::runtime::Handle;

use super::{ActionSpec, Outside, Visit};
use crate::ExtensionError;
use crate::bindings::synt::extension::types::Error;
use crate::caller::Caller;
use crate::manifest::Manifest;
use crate::runtime::{Compiled, GuestCall, Runtime};
use crate::shelf::Shelf;

const MOST_ACTIONS: usize = 64;

pub(crate) type CallerCell = Arc<OnceLock<Arc<dyn Caller>>>;

pub(crate) struct Guest {
    manifest: Manifest,
    compiled: Compiled,
    runtime: Runtime,
    shelf: Shelf,
    caller: CallerCell,
}

impl Guest {
    pub(crate) fn new(
        manifest: Manifest,
        compiled: Compiled,
        runtime: Runtime,
        shelf: Shelf,
        caller: CallerCell,
    ) -> Self {
        Self {
            manifest,
            compiled,
            runtime,
            shelf,
            caller,
        }
    }

    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub(crate) fn describe(&self) -> Result<Vec<ActionSpec>, ExtensionError> {
        let visit = Visit::new(self.manifest.clone(), self.shelf.clone(), None);
        let described = self.runtime.describe(&self.compiled, Box::new(visit))?;

        if described.is_empty() || described.len() > MOST_ACTIONS {
            return Err(ExtensionError::Component(format!(
                "an extension adds 1 to {MOST_ACTIONS} actions, this one describes {}",
                described.len()
            )));
        }

        let specs = described
            .iter()
            .map(|action| ActionSpec::from_described(&self.manifest.id, action))
            .collect::<Result<Vec<_>, _>>()?;

        check_unique(&specs)?;
        Ok(specs)
    }

    pub(crate) fn run(
        &self,
        call: &GuestCall,
        context: Context,
        runtime: Handle,
    ) -> Result<Result<String, Error>, ExtensionError> {
        let caller = self.caller.get().cloned().ok_or_else(|| {
            ExtensionError::Refused("the core is not ready to run extensions yet".to_owned())
        })?;

        let stop = context.stop().clone();
        let outside = Outside {
            caller,
            context,
            runtime,
        };
        let visit = Visit::new(self.manifest.clone(), self.shelf.clone(), Some(outside));

        self.runtime
            .run(&self.compiled, Box::new(visit), call, stop)
    }
}

fn check_unique(specs: &[ActionSpec]) -> Result<(), ExtensionError> {
    let mut seen = BTreeSet::new();

    for spec in specs {
        if !seen.insert(spec.id.as_str()) {
            return Err(ExtensionError::Component(format!(
                "action {} is described twice",
                spec.id
            )));
        }
    }

    Ok(())
}
