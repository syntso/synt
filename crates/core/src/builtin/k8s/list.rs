use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{KindName, ListQuery, Listing};

use super::{KindFacts, Kube};

use crate::CoreError;

#[derive(Debug, Clone)]
pub(crate) struct K8sList {
    kube: Kube,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct ListParams {
    pub kind: KindName,

    #[serde(flatten)]
    pub query: ListQuery,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sList {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn list(&self, context: &Context, params: &ListParams) -> Result<Listing, CoreError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &params.kind).await?;

        Ok(params.query.run(&cluster, &kind).await?)
    }
}

impl Action for K8sList {
    const ID: &'static str = "k8s.list";
    const TITLE: &'static str = "List the objects of a kind in a cluster";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ListParams;
    type Output = Listing;

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
        Ok(self.list(context, &params).await?)
    }
}
