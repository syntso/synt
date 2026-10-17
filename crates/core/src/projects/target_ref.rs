use crate::CoreError;

const PREFIX: char = '@';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetRef<'a> {
    project: &'a str,
    resource: &'a str,
}

impl<'a> TargetRef<'a> {
    pub(crate) fn parse(text: &'a str) -> Option<Result<Self, CoreError>> {
        let rest = text.strip_prefix(PREFIX)?;

        let parsed = match rest.split_once('/') {
            Some((project, resource)) if !project.is_empty() && !resource.is_empty() => {
                Ok(Self { project, resource })
            }
            _ => Err(CoreError::NotATarget(text.to_owned())),
        };

        Some(parsed)
    }

    pub(crate) fn require(text: &'a str) -> Result<Self, CoreError> {
        Self::parse(text).unwrap_or_else(|| Err(CoreError::NotAProjectResource(text.to_owned())))
    }

    pub(crate) fn project(&self) -> &'a str {
        self.project
    }

    pub(crate) fn resource(&self) -> &'a str {
        self.resource
    }
}
