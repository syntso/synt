use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{KindName, WatchRequest};
use synt_proto::StreamId;

use super::streams::{Pump, StreamStarted};
use super::{K8sActions, K8sError, KindFacts};

#[derive(Debug, Clone)]
pub(crate) struct K8sWatch {
    kube: K8sActions,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WatchParams {
    pub kind: KindName,

    #[serde(flatten)]
    pub request: WatchRequest,

    #[schemars(with = "u32")]
    pub stream: StreamId,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sWatch {
    pub(super) fn new(kube: K8sActions) -> Self {
        Self { kube }
    }

    async fn watch(
        &self,
        context: &Context,
        params: &WatchParams,
    ) -> Result<StreamStarted, K8sError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &params.kind).await?;
        let pump = Pump::open(context, params.stream)?;
        let watch = params.request.open(&cluster, &kind);

        let about = json!({ "kind": kind.to_string(), "stream": params.stream.get() });
        self.kube
            .streams()
            .start(context, Self::ID, about, move |stop| async move {
                let ended = pump.run(watch, stop).await;
                drop(cluster);
                ended
            });

        Ok(StreamStarted::new(context, params.stream))
    }
}

impl Action for K8sWatch {
    const ID: &'static str = "k8s.watch";
    const TITLE: &'static str = "Stream the changes to the objects of a kind";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;
    const FINISHES_LATER: bool = true;

    type Params = WatchParams;
    type Output = StreamStarted;

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut WatchParams,
    ) -> Result<Option<Value>, ActionError> {
        params.facts = self.kube.kind_facts(target, &params.kind)?;

        Ok(None)
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &WatchParams) -> Risk {
        if params.facts.is_some_and(|facts| facts.secret) {
            return Risk::Privileged;
        }

        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &WatchParams) -> Option<String> {
        self.kube.destination(target)
    }

    async fn run(
        &self,
        context: &Context,
        params: WatchParams,
    ) -> Result<StreamStarted, ActionError> {
        Ok(self.watch(context, &params).await?)
    }
}
