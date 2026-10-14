use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::Restarted;

use super::Kube;
use super::failure::failed;
use super::object_params::ObjectParams;

#[derive(Debug, Clone)]
pub(crate) struct K8sRolloutRestart {
    kube: Kube,
}

impl K8sRolloutRestart {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
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
        _target: Option<&ResourceRef>,
        params: &ObjectParams,
    ) -> Result<(), ActionError> {
        params.check()
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &ObjectParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ObjectParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Restarted) -> Value {
        serde_json::to_value(output).unwrap_or_default()
    }

    async fn run(&self, context: &Context, params: ObjectParams) -> Result<Restarted, ActionError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &params.kind).await?;

        cluster
            .restart(&kind, &params.name, params.namespace.as_deref())
            .await
            .map_err(failed)
    }
}
