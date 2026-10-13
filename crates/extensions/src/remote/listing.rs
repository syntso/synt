use serde::{Deserialize, Serialize};

use crate::manifest::{Capabilities, Manifest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listing {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    pub file: String,
    pub sha256: String,
    pub signature: String,

    #[serde(default)]
    pub capabilities: Capabilities,
}

impl Listing {
    pub(crate) fn manifest(&self) -> Manifest {
        Manifest {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            api: self.api.clone(),
            description: self.description.clone(),
            repository: self.repository.clone(),
            capabilities: self.capabilities.clone(),
        }
    }

    pub(crate) fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();

        [&self.id, &self.name, &self.description]
            .iter()
            .any(|field| field.to_lowercase().contains(&query))
    }
}
