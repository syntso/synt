use std::sync::{Mutex, PoisonError};

use serde_json::json;
use synt_actions::Context;
use synt_extensions::{CallFuture, Caller, ExtensionCall};

#[derive(Default)]
pub struct FakeCaller {
    calls: Mutex<Vec<ExtensionCall>>,
}

impl FakeCaller {
    pub fn calls(&self) -> Vec<ExtensionCall> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Caller for FakeCaller {
    fn call<'a>(&'a self, call: ExtensionCall, _origin: Option<&'a Context>) -> CallFuture<'a> {
        Box::pin(async move {
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(call);

            Ok(json!({ "exit": 0, "stdout": "up 3 days" }))
        })
    }
}
