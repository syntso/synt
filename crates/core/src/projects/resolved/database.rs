use synt_ssh::Target;

use super::{DatabaseResource, Resolved};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedDatabase {
    pub(crate) resource: DatabaseResource,
    pub(crate) via: Option<Target>,
}

impl ResolvedDatabase {
    pub(crate) fn describe(&self) -> String {
        let place = self.resource.describe();

        match &self.via {
            Some(host) => format!("{place} via {}", Resolved::Host(host.clone()).describe()),
            None => place,
        }
    }
}
