use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{EventLine, EventQuery};

use super::{K8sError, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sEvents {
    kube: Kube,
}

impl K8sEvents {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn events(
        &self,
        context: &Context,
        query: &EventQuery,
    ) -> Result<Vec<EventLine>, K8sError> {
        let cluster = self.kube.cluster(context).await?;

        Ok(query.run(&cluster).await?)
    }
}

impl Action for K8sEvents {
    const ID: &'static str = "k8s.events";
    const TITLE: &'static str = "Read the events of a namespace or of one object";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = EventQuery;
    type Output = Vec<EventLine>;

    fn check(&self, target: Option<&ResourceRef>, _query: &EventQuery) -> Result<(), ActionError> {
        self.kube.reach(target)?;

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, _query: &EventQuery) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _query: &EventQuery) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Vec<EventLine>) -> Value {
        json!({ "events": output.len() })
    }

    async fn run(
        &self,
        context: &Context,
        query: EventQuery,
    ) -> Result<Vec<EventLine>, ActionError> {
        Ok(self.events(context, &query).await?)
    }
}
