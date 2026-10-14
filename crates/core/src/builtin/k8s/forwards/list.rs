use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::ForwardInfo;
use crate::builtin::k8s::Kube;

#[derive(Debug, Clone)]
pub(crate) struct K8sPortForwardList {
    kube: Kube,
}

impl K8sPortForwardList {
    pub(crate) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sPortForwardList {
    const ID: &'static str = "k8s.port_forward.list";
    const TITLE: &'static str = "List the open port forwards to clusters";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<ForwardInfo>;

    fn summary(&self, output: &Vec<ForwardInfo>) -> Value {
        json!({ "forwards": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<ForwardInfo>, ActionError> {
        Ok(self.kube.forwards().list())
    }
}
