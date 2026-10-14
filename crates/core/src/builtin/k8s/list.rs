use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{ListQuery, Listing, Namespaces};

use super::failure::failed;
use super::object_names::{check_kind, check_namespace};
use super::{KindFacts, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sList {
    kube: Kube,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct ListParams {
    pub kind: String,

    #[serde(default)]
    pub namespace: Option<String>,

    #[serde(default)]
    pub all_namespaces: bool,

    #[serde(default)]
    pub selector: Option<String>,

    #[serde(default)]
    pub limit: Option<u32>,

    #[serde(default, rename = "continue")]
    pub continue_token: Option<String>,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sList {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sList {
    const ID: &'static str = "k8s.list";
    const TITLE: &'static str = "List the objects of a kind in a cluster";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ListParams;
    type Output = Listing;

    fn check(&self, _target: Option<&ResourceRef>, params: &ListParams) -> Result<(), ActionError> {
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
        params: &mut ListParams,
    ) -> Result<Option<Value>, ActionError> {
        params.facts = self.kube.kind_facts(target, &params.kind)?;

        Ok(None)
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &ListParams) -> Risk {
        if params.facts.is_some_and(|facts| facts.secret) {
            return Risk::Privileged;
        }

        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ListParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Listing) -> Value {
        json!({
            "kind": output.kind,
            "items": output.items.len(),
            "more": output.continue_token.is_some(),
        })
    }

    async fn run(&self, context: &Context, params: ListParams) -> Result<Listing, ActionError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &params.kind).await?;

        let query = ListQuery {
            namespaces: Namespaces::from_request(params.namespace, params.all_namespaces),
            selector: params.selector,
            limit: params.limit,
            continue_token: params.continue_token,
        };

        cluster.list(&kind, &query).await.map_err(failed)
    }
}
