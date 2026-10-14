use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentKind {
    Project,
    Resource,
    Runbook,
    Extension { extension: String, suffix: String },
}

impl DocumentKind {
    pub fn is_core(&self) -> bool {
        !matches!(self, Self::Extension { .. })
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => formatter.write_str("core.project"),
            Self::Resource => formatter.write_str("core.resource"),
            Self::Runbook => formatter.write_str("core.runbook"),
            Self::Extension { extension, suffix } => {
                write!(formatter, "ext.{extension}.{suffix}")
            }
        }
    }
}
