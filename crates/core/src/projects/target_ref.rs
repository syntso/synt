use std::fmt;

use super::{ProjectSlug, ResourceName};
use crate::CoreError;

const PREFIX: char = '@';

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetRef {
    project: ProjectSlug,
    resource: ResourceName,
}

impl TargetRef {
    pub(crate) fn parse(text: &str) -> Option<Result<Self, CoreError>> {
        let rest = text.strip_prefix(PREFIX)?;

        let parsed = match rest.split_once('/') {
            Some((project, resource)) if !project.is_empty() && !resource.is_empty() => {
                Self::of_parts(project, resource)
            }
            _ => Err(CoreError::NotATarget(text.to_owned())),
        };

        Some(parsed)
    }

    pub(crate) fn require(text: &str) -> Result<Self, CoreError> {
        Self::parse(text).unwrap_or_else(|| Err(CoreError::NotAProjectResource(text.to_owned())))
    }

    pub(crate) fn project(&self) -> &ProjectSlug {
        &self.project
    }

    pub(crate) fn resource(&self) -> &ResourceName {
        &self.resource
    }

    fn of_parts(project: &str, resource: &str) -> Result<Self, CoreError> {
        Ok(Self {
            project: ProjectSlug::named(project)?,
            resource: ResourceName::parse(resource)?,
        })
    }
}

impl fmt::Display for TargetRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{PREFIX}{}/{}", self.project, self.resource)
    }
}
