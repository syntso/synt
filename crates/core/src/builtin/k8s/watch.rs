use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{Namespaces, WatchRequest};
use synt_proto::StreamId;

use super::names::{check_kind, check_namespace};
use super::streams::{Pump, StreamStarted};
use super::{KindFacts, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sWatch {
    kube: Kube,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WatchParams {
    pub kind: String,

    #[serde(default)]
    pub namespace: Option<String>,

    #[serde(default)]
    pub all_namespaces: bool,

    #[serde(default)]
    pub selector: Option<String>,

    #[schemars(with = "u32")]
    pub stream: StreamId,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sWatch {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
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

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &WatchParams,
    ) -> Result<(), ActionError> {
        check_kind(&params.kind)?;
        check_namespace(params.namespace.as_deref())?;

        if params.all_namespaces && params.namespace.is_some() {
            return Err(ActionError::InvalidParams(
                "give a namespace or all_namespaces, not both".to_owned(),
            ));
        }

        Ok(())
    }

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
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &params.kind).await?;
        let pump = Pump::open(context, params.stream)?;

        let request = WatchRequest {
            namespaces: Namespaces::from_request(params.namespace, params.all_namespaces),
            selector: params.selector,
        };
        let watch = cluster.watch(&kind, &request);

        let about = json!({ "kind": kind.to_string(), "stream": params.stream.0 });
        self.kube
            .streams()
            .start(context, Self::ID, about, move |stop| async move {
                let ended = pump.run(watch, stop).await;
                drop(cluster);
                ended
            });

        Ok(StreamStarted {
            request: context.request().get(),
            stream: params.stream.0,
        })
    }
}
