use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::Restarted;

use super::{K8sActions, ObjectParams};

use crate::CoreError;

#[derive(Debug, Clone)]
pub(crate) struct K8sRolloutRestart {
    kube: K8sActions,
}

impl K8sRolloutRestart {
    pub(super) fn new(kube: K8sActions) -> Self {
        Self { kube }
    }

    async fn restart(
        &self,
        context: &Context,
        object: &ObjectParams,
    ) -> Result<Restarted, CoreError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &object.kind).await?;

        Ok(object.on(&cluster, &kind).restart().await?)
    }
}

impl Action for K8sRolloutRestart {
    const ID: &'static str = "k8s.rollout_restart";
    const TITLE: &'static str = "Restart the pods of a deployment, stateful set or daemon set";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ObjectParams;
    type Output = Restarted;

    fn check(
        &self,
        target: Option<&ResourceRef>,
        _object: &ObjectParams,
    ) -> Result<(), ActionError> {
        self.kube.reach(target)?;

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, _object: &ObjectParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _object: &ObjectParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Restarted) -> Value {
        serde_json::to_value(output).unwrap_or_default()
    }

    async fn run(&self, context: &Context, object: ObjectParams) -> Result<Restarted, ActionError> {
        Ok(self.restart(context, &object).await?)
    }
}
