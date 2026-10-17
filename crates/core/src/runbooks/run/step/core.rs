use std::sync::Arc;

use synt_actions::Context;
use synt_journal::{Actor, Seq};
use synt_proto::{ActionCall, ActionOutcome, RpcError};
use tokio_util::sync::CancellationToken;

use super::StepCaller;
use crate::core::Core;
use crate::link::Link;
use crate::on_behalf::OnBehalf;
use crate::runbooks::plan::PlannedStep;

pub(crate) struct CoreSteps {
    core: Arc<Core>,
    link: Arc<Link>,
    runbook: Actor,
    starter: Actor,
    run: Seq,
    approved: bool,
    stop: CancellationToken,
}

impl CoreSteps {
    pub(crate) fn new(
        core: Arc<Core>,
        link: Arc<Link>,
        runbook: Actor,
        context: &Context,
        stop: CancellationToken,
    ) -> Self {
        Self {
            core,
            link,
            runbook,
            starter: context.actor().clone(),
            run: context.request(),
            approved: false,
            stop,
        }
    }

    pub(crate) fn approved(self, approved: bool) -> Self {
        Self { approved, ..self }
    }
}

impl StepCaller for CoreSteps {
    async fn call(&self, step: &PlannedStep) -> Result<ActionOutcome, RpcError> {
        let call = ActionCall {
            action: step.action().as_str().to_owned(),
            target: step.target_text(),
            params: step.params().clone(),
        };

        let origin = OnBehalf {
            actor: &self.starter,
            request: self.run,
            approved: self.approved,
            planned: step.destination(),
            stop: Some(&self.stop),
        };

        self.core
            .dispatch_on_behalf(&self.runbook, origin, call, &self.stop, &self.link)
            .await
    }
}
