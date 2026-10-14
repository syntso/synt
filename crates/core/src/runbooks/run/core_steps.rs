use std::sync::Arc;

use synt_journal::{Actor, Seq};
use synt_proto::{ActionCall, ActionOutcome, RpcError};
use tokio_util::sync::CancellationToken;

use super::StepCaller;
use crate::core::Core;
use crate::link::Link;
use crate::on_behalf::OnBehalf;
use crate::runbooks::plan::PlannedStep;

pub(crate) struct CoreSteps {
    pub(crate) core: Arc<Core>,
    pub(crate) link: Arc<Link>,
    pub(crate) runbook: Actor,
    pub(crate) starter: Actor,
    pub(crate) run: Seq,
    pub(crate) approved: bool,
    pub(crate) stop: CancellationToken,
}

impl StepCaller for CoreSteps {
    async fn call(&self, step: &PlannedStep) -> Result<ActionOutcome, RpcError> {
        let call = ActionCall {
            action: step.action.as_str().to_owned(),
            target: step.target_text(),
            params: step.params.clone(),
        };

        let origin = OnBehalf {
            actor: &self.starter,
            request: self.run,
            approved: self.approved,
            stop: Some(&self.stop),
        };

        self.core
            .dispatch_on_behalf(&self.runbook, origin, call, &self.stop, &self.link)
            .await
    }
}
