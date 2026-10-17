use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::Seq;

use crate::builtin::k8s::K8sActions;

#[derive(Debug, Clone)]
pub(crate) struct K8sStreamClose {
    kube: K8sActions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct StreamCloseParams {
    #[schemars(with = "u64")]
    pub request: Seq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StreamClosed {
    pub request: Seq,
}

impl K8sStreamClose {
    pub(crate) fn new(kube: K8sActions) -> Self {
        Self { kube }
    }
}

impl Action for K8sStreamClose {
    const ID: &'static str = "k8s.stream.close";
    const TITLE: &'static str = "Stop following logs or watching a kind";
    const RISK: Risk = Risk::Read;

    type Params = StreamCloseParams;
    type Output = StreamClosed;

    fn summary(&self, output: &StreamClosed) -> Value {
        json!({ "request": output.request })
    }

    async fn run(
        &self,
        context: &Context,
        params: StreamCloseParams,
    ) -> Result<StreamClosed, ActionError> {
        self.kube.streams().close(params.request, context.actor())?;

        Ok(StreamClosed {
            request: params.request,
        })
    }
}
