use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_kube::ContextInfo;

use super::Kube;

#[derive(Debug, Clone)]
pub(crate) struct K8sContexts {
    kube: Kube,
}

impl K8sContexts {
    pub(super) fn new(kube: Kube) -> Self {
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
        self.kube.contexts().await
    }
}
