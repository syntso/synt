use super::{
    ProjectInfo, ProjectPlace, Resolved, ResolvedDatabase, Resource, ResourceEntry, ResourceListing,
};
use crate::CoreError;

pub(crate) trait ProjectSource {
    fn info(&self) -> Result<ProjectInfo, CoreError>;

    fn listing(&self) -> Result<ResourceListing, CoreError>;

    fn resource(&self, name: &str) -> Result<ResourceEntry, CoreError>;

    fn resolve(&self, name: &str) -> Result<Resolved, CoreError>;

    fn database(&self, name: &str) -> Result<ResolvedDatabase, CoreError>;

    fn put(
        &mut self,
        name: &str,
        admitted: Option<&str>,
        resource: Resource,
    ) -> Result<ResourceEntry, CoreError>;

    fn remove(&mut self, name: &str, admitted: Option<&str>) -> Result<ResourceEntry, CoreError>;

    fn place(&self) -> Result<ProjectPlace, CoreError>;

    fn canonical_name(&self) -> String;
}
