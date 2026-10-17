mod cluster;
mod database;
mod entry;
mod folder;
mod host;
mod listing;
mod name;
mod spec;
mod tunnel;

pub use cluster::ClusterResource;

pub use database::DatabaseResource;
pub use entry::ResourceEntry;
pub(crate) use folder::ResourceFolder;
pub use host::HostResource;
pub use listing::ResourceListing;
pub(crate) use name::ResourceName;
pub use tunnel::TunnelResource;

use spec::ResourceSpec;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

    pub(crate) fn to_toml(&self) -> Result<String, CoreError> {
        Ok(toml::to_string(self)?)
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        self.spec().check()
    }

    pub(crate) fn kind(&self) -> &'static str {
        self.spec().kind()
    }

    fn spec(&self) -> &dyn ResourceSpec {
        match self {
            Self::Host(host) => host,
            Self::Tunnel(tunnel) => tunnel,
            Self::Cluster(cluster) => cluster,
            Self::Database(database) => database,
        }
    }
}
