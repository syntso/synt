use std::future::Future;

use synt_proto::{ActionOutcome, RpcError};

use crate::runbooks::plan::PlannedStep;

pub(crate) trait StepCaller: Send + Sync {
    fn call(
        &self,
        step: &PlannedStep,
    ) -> impl Future<Output = Result<ActionOutcome, RpcError>> + Send;
}
