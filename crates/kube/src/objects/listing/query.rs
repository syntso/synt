use kube::api::ListParams;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{Cluster, Kind, KubeError, Listing, NamespaceScope};

const LARGEST_PAGE: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
pub struct ListQuery {
    #[serde(flatten)]
    pub scope: NamespaceScope,

    #[serde(default)]
    pub selector: Option<String>,

    #[serde(default)]
    pub limit: Option<u32>,

    #[serde(default, rename = "continue")]
    pub continue_token: Option<String>,
}

impl ListQuery {
    pub async fn run(&self, cluster: &Cluster, kind: &Kind) -> Result<Listing, KubeError> {
        let found = cluster
            .objects(kind, &self.scope)
            .list(&self.params())
            .await?;

        Ok(Listing {
            kind: kind.to_string(),
            items: found
                .items
                .into_iter()
                .map(|object| kind.tidy(object))
                .collect(),
            continue_token: found.metadata.continue_.filter(|token| !token.is_empty()),
        })
    }

    fn params(&self) -> ListParams {
        let limit = self.limit.unwrap_or(LARGEST_PAGE).clamp(1, LARGEST_PAGE);
        let mut params = ListParams::default().limit(limit);

        if let Some(selector) = &self.selector {
            params = params.labels(selector);
        }

        if let Some(token) = &self.continue_token {
            params = params.continue_token(token);
        }

        params
    }
}
