use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::{Api, Capabilities, Contributions, Manifest};
use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Listing {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    pub file: String,
    pub sha256: Sha256,
    pub signature: String,

    #[serde(default)]
    pub capabilities: Capabilities,

    #[serde(default, skip_serializing_if = "Contributions::is_empty")]
    pub contributes: Contributions,
}

impl Listing {
    pub(crate) fn manifest(&self) -> Result<Manifest, ExtensionError> {
        let api = Api::parse(&self.api).map_err(ExtensionError::Manifest)?;

        Ok(Manifest {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            api,
            description: self.description.clone(),
            repository: self.repository.clone(),
            capabilities: self.capabilities.clone(),
            contributes: self.contributes.clone(),
        })
    }

    pub(crate) fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();

        [&self.id, &self.name, &self.description]
            .iter()
            .any(|field| field.to_lowercase().contains(&query))
    }
}
