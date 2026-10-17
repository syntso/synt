use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{ForwardId, ForwardInfo};
use crate::CoreError;
use crate::builtin::k8s::Kube;

#[derive(Debug, Clone)]
pub(crate) struct K8sPortForwardClose {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ForwardCloseParams {
    pub id: ForwardId,
}

impl K8sPortForwardClose {
    pub(crate) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sPortForwardClose {
    const ID: &'static str = "k8s.port_forward.close";
    const TITLE: &'static str = "Close a port forward to a cluster";
    const RISK: Risk = Risk::Write;

    type Params = ForwardCloseParams;
    type Output = ForwardInfo;

    fn risk(&self, _target: Option<&ResourceRef>, params: &ForwardCloseParams) -> Risk {
        self.kube
            .forwards()
            .risk(params.id)
            .unwrap_or(Self::RISK)
            .max(Self::RISK)
    }

    fn summary(&self, output: &ForwardInfo) -> Value {
        json!({
            "id": output.id,
            "local_port": output.status.local_port,
            "connections": output.status.connections,
            "sent_bytes": output.status.sent_bytes,
            "received_bytes": output.status.received_bytes,
        })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ForwardCloseParams,
    ) -> Result<ForwardInfo, ActionError> {
        let closed = self.kube.forwards().close(params.id).await;

        Ok(closed.ok_or(CoreError::NoForward(params.id))?)
    }
}
