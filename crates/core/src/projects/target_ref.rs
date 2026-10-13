const PREFIX: char = '@';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetRef<'a> {
    pub(crate) project: &'a str,
    pub(crate) resource: &'a str,
}

impl<'a> TargetRef<'a> {
    pub(crate) fn parse(text: &'a str) -> Option<Result<Self, String>> {
        let rest = text.strip_prefix(PREFIX)?;

        let parsed = match rest.split_once('/') {
            Some((project, resource)) if !project.is_empty() && !resource.is_empty() => {
                Ok(Self { project, resource })
            }
            _ => Err(format!(
                "{text:?} is not a project target; write @<project>/<resource>"
            )),
        };

        Some(parsed)
    }
}
