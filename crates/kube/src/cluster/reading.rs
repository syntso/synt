use k8s_openapi::api::core::v1::Event;
use kube::api::{Api, ListParams};
use serde_json::Value;

use super::Cluster;
use crate::{EventLine, Kind, KubeError, ListQuery, Listing, Namespaces};

const LARGEST_PAGE: u32 = 500;

impl Cluster {
    pub async fn list(&self, kind: &Kind, query: &ListQuery) -> Result<Listing, KubeError> {
        let namespace = self.namespace_among(kind, &query.namespaces);
        let api = kind.api(self.client(), namespace.as_deref());

        let mut params =
            ListParams::default().limit(query.limit.unwrap_or(LARGEST_PAGE).clamp(1, LARGEST_PAGE));
        if let Some(selector) = &query.selector {
            params = params.labels(selector);
        }
        if let Some(token) = &query.continue_token {
            params = params.continue_token(token);
        }

        let list = api.list(&params).await?;
        let continue_token = list.metadata.continue_.filter(|token| !token.is_empty());

        Ok(Listing {
            kind: kind.to_string(),
            items: list
                .items
                .into_iter()
                .map(|object| kind.tidy(object))
                .collect(),
            continue_token,
        })
    }

    pub async fn get(
        &self,
        kind: &Kind,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<Value, KubeError> {
        let api = kind.api(
            self.client(),
            self.namespace_for(kind, namespace).as_deref(),
        );
        let object = api.get(name).await?;

        Ok(kind.tidy(object))
    }

    pub async fn events(
        &self,
        namespaces: &Namespaces,
        object: Option<&str>,
    ) -> Result<Vec<EventLine>, KubeError> {
        let api: Api<Event> = match self.namespace_named(namespaces) {
            Some(namespace) => Api::namespaced(self.client().clone(), &namespace),
            None => Api::all(self.client().clone()),
        };

        let mut params = ListParams::default().limit(LARGEST_PAGE);
        if let Some(name) = object {
            params = params.fields(&format!("involvedObject.name={name}"));
        }

        let mut lines: Vec<EventLine> = api
            .list(&params)
            .await?
            .items
            .into_iter()
            .map(EventLine::from)
            .collect();
        lines.sort_by(|first, second| first.last_seen.cmp(&second.last_seen));

        Ok(lines)
    }
}
