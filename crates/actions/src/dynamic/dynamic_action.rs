use std::future::Future;

use serde_json::Value;
use synt_journal::ResourceRef;

use crate::{ActionError, Context, Risk};

pub trait DynamicAction: Send + Sync + 'static {
    fn title(&self) -> &str;

    fn risk(&self) -> Risk;

    fn takes_target(&self) -> bool;

    fn params_schema(&self) -> Value;

    fn destination(&self, _target: Option<&ResourceRef>) -> Option<String> {
        None
    }

    fn run(
        &self,
        context: &Context,
        target: Option<ResourceRef>,
        params: Value,
    ) -> impl Future<Output = Result<Value, ActionError>> + Send;
}
