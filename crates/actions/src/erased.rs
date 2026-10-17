use std::sync::Arc;

use serde_json::Value;
use synt_journal::ResourceRef;

use crate::{ActionError, Prepared, Risk};

pub(crate) trait Erased: Send + Sync {
    fn title(&self) -> &str;

    fn declared_risk(&self) -> Risk;

    fn person_only(&self) -> bool;

    fn takes_target(&self) -> bool;

    fn params_schema(&self) -> Value;

    fn recorded_params(&self, params: &Value) -> Value;

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError>;
}
