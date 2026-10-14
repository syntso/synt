use std::fmt::{self, Display, Formatter};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ClusterPlace {
    pub kubeconfig: String,
    pub context: String,
    pub server: String,
    pub namespace: String,
}

impl Display for ClusterPlace {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "context {} at {} in {} (namespace {})",
            self.context, self.server, self.kubeconfig, self.namespace
        )
    }
}
