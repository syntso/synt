use synt_sync::ProjectFolder;

use super::{
    ProjectInfo, Resolved, ResolvedDatabase, Resource, ResourceEntry, ResourceListing, ResourceName,
};
use crate::CoreError;

pub(crate) trait ProjectSource {
    fn info(&self) -> Result<ProjectInfo, CoreError>;

    fn listing(&self) -> Result<ResourceListing, CoreError>;

    fn resource(&self, name: &ResourceName) -> Result<ResourceEntry, CoreError>;

    fn resolve(&self, name: &ResourceName) -> Result<Resolved, CoreError>;

    fn database(&self, name: &ResourceName) -> Result<ResolvedDatabase, CoreError>;

    fn put(
        &mut self,
        name: &ResourceName,
        admitted: Option<&str>,
        resource: Resource,
    ) -> Result<ResourceEntry, CoreError>;

    fn remove(
        &mut self,
        name: &ResourceName,
        admitted: Option<&str>,
    ) -> Result<ResourceEntry, CoreError>;

    fn place(&self) -> Result<&ProjectFolder, CoreError>;

    fn canonical_name(&self) -> String;
}
