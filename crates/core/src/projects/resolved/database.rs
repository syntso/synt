use std::fmt;

use synt_ssh::Target;

use super::login_label;
use crate::projects::DatabaseResource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedDatabase {
    pub(crate) resource: DatabaseResource,
    pub(crate) via: Option<Target>,
}

impl fmt::Display for ResolvedDatabase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.resource)?;

        match &self.via {
            Some(host) => write!(formatter, " via {}", login_label(host)),
            None => Ok(()),
        }
    }
}
