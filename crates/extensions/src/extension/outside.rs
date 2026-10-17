use std::sync::Arc;

use synt_actions::Context;
use tokio::runtime::Handle;

use crate::call::{CallFailure, Caller, ExtensionCall};

pub(crate) struct Outside {
    caller: Arc<dyn Caller>,
    origin: Option<Context>,
    runtime: Handle,
}

impl Outside {
    pub(crate) fn new(caller: Arc<dyn Caller>, origin: Option<Context>, runtime: Handle) -> Self {
        Self {
            caller,
            origin,
            runtime,
        }
    }

    pub(crate) fn call(&self, call: ExtensionCall) -> Result<serde_json::Value, CallFailure> {
        self.runtime
            .block_on(self.caller.call(call, self.origin.as_ref()))
    }
}
