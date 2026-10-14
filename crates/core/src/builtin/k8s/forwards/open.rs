use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::ForwardTarget;

use super::{ForwardInfo, OpenForward};
use crate::builtin::hold::keep_hold_while_open;
use crate::builtin::k8s::Kube;
use crate::builtin::k8s::failure::failed;
use crate::builtin::k8s::names::{check_namespace, check_object_name};

#[derive(Debug, Clone)]
pub(crate) struct K8sPortForward {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct PortForwardParams {
    #[serde(default)]
    pub pod: Option<String>,

    #[serde(default)]
    pub service: Option<String>,

    #[serde(default)]
    pub namespace: Option<String>,

    pub port: u16,

    #[serde(default)]
    pub local_port: Option<u16>,
}

impl PortForwardParams {
    fn target(&self) -> Result<ForwardTarget, ActionError> {
        match (&self.pod, &self.service) {
            (Some(pod), None) => Ok(ForwardTarget::Pod(pod.clone())),
            (None, Some(service)) => Ok(ForwardTarget::Service(service.clone())),
            _ => Err(ActionError::InvalidParams(
                "name either a pod or a service to forward to".to_owned(),
            )),
        }
    }
}

impl K8sPortForward {
    pub(crate) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sPortForward {
    const ID: &'static str = "k8s.port_forward";
    const TITLE: &'static str = "Forward a local port to a port of a pod or service";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = PortForwardParams;
    type Output = ForwardInfo;

    fn check(
        &self,
        target: Option<&ResourceRef>,
        params: &PortForwardParams,
    ) -> Result<(), ActionError> {
        self.kube.reach(target)?;

        let name = match params.target()? {
            ForwardTarget::Pod(name) | ForwardTarget::Service(name) => name,
        };
        check_object_name("the pod or service", &name)?;
        check_namespace(params.namespace.as_deref())?;

        if params.port == 0 || params.local_port == Some(0) {
            return Err(ActionError::InvalidParams(
                "ports must be between 1 and 65535; leave local_port out to pick a free one"
                    .to_owned(),
            ));
        }

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &PortForwardParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(
        &self,
        target: Option<&ResourceRef>,
        _params: &PortForwardParams,
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
        params: PortForwardParams,
    ) -> Result<ForwardInfo, ActionError> {
        let target = params.target()?;
        let cluster = self.kube.cluster(context).await?;

        let forward = cluster
            .forward(
                &target,
                params.namespace.as_deref(),
                params.port,
                params.local_port,
            )
            .await
            .map_err(failed)?;

        let stop = context.stop().clone();
        let open = OpenForward {
            forward,
            cluster: context.destination().unwrap_or_default().to_owned(),
            risk: context.risk(),
            stop: stop.clone(),
            _connection: cluster,
        };
        let info = self.kube.forwards().add(open);

        keep_hold_while_open(context, stop);

        Ok(info)
    }
}
