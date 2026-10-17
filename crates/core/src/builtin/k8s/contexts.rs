use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_kube::ContextInfo;

use super::K8sActions;

#[derive(Debug, Clone)]
pub(crate) struct K8sContexts {
    kube: K8sActions,
}

impl K8sContexts {
    pub(super) fn new(kube: K8sActions) -> Self {
        Self { kube }
    }
}

impl Action for K8sContexts {
    const ID: &'static str = "k8s.contexts";
    const TITLE: &'static str = "List the contexts of the kubeconfig";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<ContextInfo>;

    fn summary(&self, output: &Vec<ContextInfo>) -> Value {
        json!({ "contexts": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<ContextInfo>, ActionError> {
        Ok(self.kube.contexts().await?)
    }
}
