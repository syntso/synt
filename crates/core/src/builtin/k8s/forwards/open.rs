use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::ForwardRequest;

use super::{ForwardEntry, ForwardInfo};
use crate::builtin::hold::keep_hold_while_open;
use crate::builtin::k8s::{K8sError, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sPortForward {
    kube: Kube,
}

impl K8sPortForward {
    pub(crate) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn open(
        &self,
        context: &Context,
        request: &ForwardRequest,
    ) -> Result<ForwardInfo, K8sError> {
        let cluster = self.kube.cluster(context).await?;
        let forward = request.open(&cluster).await?;

        let entry = ForwardEntry::new(forward, context, cluster);
        let stop = entry.stop().clone();
        let info = self.kube.forwards().add(entry);

        keep_hold_while_open(context, stop);

        Ok(info)
    }
}

impl Action for K8sPortForward {
    const ID: &'static str = "k8s.port_forward";
    const TITLE: &'static str = "Forward a local port to a port of a pod or service";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ForwardRequest;
    type Output = ForwardInfo;

    fn check(
        &self,
        target: Option<&ResourceRef>,
        _params: &ForwardRequest,
    ) -> Result<(), ActionError> {
        self.kube.reach(target)?;

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &ForwardRequest) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(
        &self,
        target: Option<&ResourceRef>,
        _params: &ForwardRequest,
    ) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &ForwardInfo) -> Value {
        json!({
            "id": output.id,
            "local_port": output.status.local_port,
            "pod": output.status.pod,
            "namespace": output.status.namespace,
            "remote_port": output.status.remote_port,
        })
    }

    async fn run(
        &self,
        context: &Context,
        request: ForwardRequest,
    ) -> Result<ForwardInfo, ActionError> {
        Ok(self.open(context, &request).await?)
    }
}
