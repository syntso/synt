mod address;
mod cluster;
mod context_name;
mod database;
mod entry;
mod folder;
mod host;
mod identity;
mod listing;
mod login;
mod name;
mod tunnel;

pub(crate) use address::Address;
pub use cluster::ClusterResource;
pub(crate) use context_name::ContextName;
pub use database::DatabaseResource;
pub use entry::ResourceEntry;
pub(crate) use folder::ResourceFolder;
pub use host::HostResource;
pub(crate) use identity::Identity;
pub use listing::ResourceListing;
pub(crate) use login::Login;
pub(crate) use name::ResourceName;
pub use tunnel::TunnelResource;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resource {
    Host(HostResource),
    Tunnel(TunnelResource),
    Cluster(ClusterResource),
    Database(DatabaseResource),
}

impl Resource {
    pub(crate) fn from_toml(text: &str) -> Result<Self, CoreError> {
        let resource: Self = toml::from_str(text)?;

        resource.check()?;
        Ok(resource)
    }

    pub(crate) fn from_fields(fields: Map<String, Value>) -> Result<Self, CoreError> {
        let resource: Self =
            serde_json::from_value(Value::Object(fields)).map_err(CoreError::ResourceFields)?;

        resource.check()?;
        Ok(resource)
    }

    pub(crate) fn to_toml(&self) -> Result<String, CoreError> {
        Ok(toml::to_string(self)?)
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        match self {
            Self::Database(database) => database.check(),
            Self::Host(_) | Self::Tunnel(_) | Self::Cluster(_) => Ok(()),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Host(_) => "host",
            Self::Tunnel(_) => "tunnel",
            Self::Cluster(_) => "cluster",
            Self::Database(_) => "database",
        }
    }
}
